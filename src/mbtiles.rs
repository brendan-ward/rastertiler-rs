use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use seahash::hash;

use crate::tileid::TileID;

const INIT_QUERY: &str = r#"
PRAGMA journal_mode=WAL;

CREATE TABLE IF NOT EXISTS metadata (name text NOT NULL PRIMARY KEY, value text);

CREATE TABLE IF NOT EXISTS map (
    zoom_level INTEGER,
    tile_column INTEGER,
    tile_row INTEGER,
    tile_id sqlite3_int64
);

CREATE TABLE IF NOT EXISTS images (tile_id sqlite3_int64 NOT NULL PRIMARY KEY, tile_data blob);

CREATE VIEW IF NOT EXISTS tiles AS
    SELECT zoom_level, tile_column, tile_row, tile_data
    FROM map JOIN images ON images.tile_id = map.tile_id;
"#;

const INSERT_METADATA_QUERY: &str = "INSERT INTO metadata (name,value) VALUES (?, ?)";
const INSERT_TILE_DATA_QUERY: &str =
    "INSERT OR IGNORE INTO images (tile_id, tile_data) VALUES (?, ?)";
const INSERT_TILE_QUERY: &str =
    "INSERT INTO map (zoom_level, tile_column, tile_row, tile_id) VALUES(?, ?, ?, ?)";

const UPDATE_INDEX_QUERY: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS map_index ON map (zoom_level, tile_column, tile_row);

PRAGMA wal_checkpoint(TRUNCATE);
"#;

const RESET_WAL_QUERY: &str = "PRAGMA journal_mode=DELETE";

pub struct MBTiles {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl MBTiles {
    pub fn new(path: &PathBuf, pool_size: u8) -> Result<MBTiles> {
        // always overwrite existing database
        if path.exists() {
            fs::remove_file(path)?;
        }

        let manager =
            SqliteConnectionManager::file(path).with_init(|c| c.execute_batch(INIT_QUERY));

        let pool = r2d2::Pool::builder()
            .max_size(pool_size as u32)
            .build(manager)?;

        Ok(MBTiles { pool })
    }

    pub fn open(path: &PathBuf, pool_size: u8) -> Result<MBTiles> {
        let manager = SqliteConnectionManager::file(path); //.with_init(|c| c.execute_batch(INIT_QUERY));

        let pool = r2d2::Pool::builder()
            .max_size(pool_size as u32)
            .build(manager)?;

        Ok(MBTiles { pool })
    }

    pub fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        Ok(self.pool.get()?)
    }

    pub fn set_metadata(&self, metadata: &[(&str, &str)]) -> Result<(), Box<dyn Error>> {
        let mut conn = self.pool.get().unwrap();
        let tx = conn.transaction()?;

        // run queries in a block so that query goes out of scope before tx
        {
            let mut query = tx.prepare(INSERT_METADATA_QUERY)?;
            for &(key, value) in metadata.iter() {
                query.execute([key, value])?;
            }
        }

        tx.commit()?;

        Ok(())
    }

    pub fn write_tile(
        &self,
        conn: &PooledConnection<SqliteConnectionManager>,
        tile_id: &TileID,
        png_data: &[u8],
    ) -> Result<()> {
        let id = hash(png_data) as i64;

        let mut query = conn.prepare_cached(INSERT_TILE_DATA_QUERY)?;
        query.execute(params![id, png_data])?;

        let mut query = conn.prepare_cached(INSERT_TILE_QUERY)?;

        // flip tile Y to match mbtiles spec
        let y = (1u32 << tile_id.zoom as u32) - 1u32 - tile_id.y;
        query.execute(params![tile_id.zoom, tile_id.x, y, id])?;

        Ok(())
    }

    pub fn update_index(&self) -> Result<()> {
        let conn = self.pool.get().unwrap();
        conn.execute_batch(UPDATE_INDEX_QUERY)?;

        Ok(())
    }

    pub fn flush(path: &PathBuf) -> Result<()> {
        let conn = Connection::open(path)?;
        conn.execute_batch(RESET_WAL_QUERY)?;

        // delete -wal and -shm files if exist
        let path_str = path.to_str().unwrap();
        let mut shm_path = PathBuf::new();
        shm_path.push(format!("{}-shm", path_str));
        if shm_path.exists() {
            fs::remove_file(shm_path)?;
        }

        let mut wal_path = PathBuf::new();
        wal_path.push(format!("{}-wal", path_str));
        if wal_path.exists() {
            fs::remove_file(wal_path)?;
        }

        Ok(())
    }
}

pub fn merge(left: &PathBuf, right: &Path, out: &PathBuf) -> Result<()> {
    // copy left to output
    fs::copy(left, out)?;

    let out_mbtiles = MBTiles::open(out, 1).unwrap();
    let mut conn = out_mbtiles.get_connection().unwrap();

    // make sure index exists so that we insert and ignore existing records
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS map_index ON map (zoom_level, tile_column, tile_row);",
        (),
    )?;

    conn.execute(
        format!("attach '{:}' as source;", right.to_str().unwrap()).as_str(),
        (),
    )?;

    let tx = conn.transaction()?;

    {
        // merge tile indexes
        tx.execute(
            "INSERT or IGNORE INTO map(zoom_level, tile_column, tile_row, tile_id) SELECT zoom_level, tile_column, tile_row, tile_id from source.map;",
            (),
        )?;

        // merge tile data
        tx.execute(
            "INSERT or IGNORE INTO images SELECT * from source.images;",
            (),
        )?;

        // update metadata for zoom levels
        tx.execute(r#"
            with min_value as(
                with combined as (
                    select cast (value as INTEGER) as value from source.metadata where name="minzoom"
                    UNION
                    select cast (value as INTEGER) as value from metadata where name="minzoom"
                )
                select min(value) as new_value from combined
            )
            update metadata
            set value = cast(min_value.new_value as TEXT)
            from min_value
            where name="minzoom";

            with max_value as(
                with combined as (
                    select cast (value as INTEGER) as value from source.metadata where name="maxzoom"
                    UNION
                    select cast (value as INTEGER) as value from metadata where name="maxzoom"
                )
                select max(value) as new_value from combined
            )
            update metadata
            set value = cast(max_value.new_value as TEXT)
            from max_value
            where name="maxzoom";
        "#, ())?;
    }

    tx.commit()?;
    conn.execute("detach source;", ())?;

    conn.execute_batch(
        r#"
        VACUUM;
        PRAGMA optimize;
    "#,
    )?;

    Ok(())
}
