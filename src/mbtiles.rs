use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::result::Result;

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use rusqlite::Connection;
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

// PRAGMA journal_mode=DELETE;
const CLOSE_MBTILES_QUERY: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS map_index ON map (zoom_level, tile_column, tile_row);

PRAGMA wal_checkpoint(TRUNCATE);
"#;

const RESET_WAL_QUERY: &str = "PRAGMA journal_mode=DELETE";

pub struct MBTiles {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl MBTiles {
    pub fn new(path: &PathBuf, pool_size: u8) -> Result<MBTiles, Box<dyn Error>> {
        // always overwrite existing database
        if path.exists() {
            fs::remove_file(path)?;
        }

        let manager =
            SqliteConnectionManager::file(path).with_init(|c| c.execute_batch(&INIT_QUERY));

        let pool = r2d2::Pool::builder()
            .max_size(pool_size as u32)
            .build(manager)?;

        return Ok(MBTiles { pool: pool });
    }

    pub fn get_connection(
        &self,
    ) -> Result<PooledConnection<SqliteConnectionManager>, Box<dyn Error>> {
        Ok(self.pool.get()?)
    }

    pub fn set_metadata(&self, metadata: &Vec<(&str, &str)>) -> Result<(), Box<dyn Error>> {
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
    ) -> Result<(), Box<dyn Error>> {
        let id = hash(png_data) as i64;

        let mut query = conn.prepare_cached(INSERT_TILE_DATA_QUERY)?;
        query.execute(params![id, png_data])?;

        let mut query = conn.prepare_cached(INSERT_TILE_QUERY)?;

        // flip tile Y to match mbtiles spec
        let y = (1u32 << tile_id.zoom as u32) - 1u32 - tile_id.y;
        query.execute(params![tile_id.zoom, tile_id.x, y, id])?;

        Ok(())
    }

    pub fn close(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.pool.get().unwrap();
        conn.execute_batch(CLOSE_MBTILES_QUERY)?;

        Ok(())
    }

    pub fn flush(path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(path)?;
        conn.execute_batch(&RESET_WAL_QUERY)?;

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
