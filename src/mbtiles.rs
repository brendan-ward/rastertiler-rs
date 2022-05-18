use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::result::Result;

use crypto::digest::Digest;
use crypto::sha1::Sha1;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

use crate::tileid::TileID;

// TODO: pragmas
const INIT_QUERY: &str = r#"
CREATE TABLE IF NOT EXISTS metadata (name text NOT NULL PRIMARY KEY, value text);

CREATE TABLE IF NOT EXISTS map (
    zoom_level INTEGER,
    tile_column INTEGER,
    tile_row INTEGER,
    tile_id TEXT
);

CREATE TABLE IF NOT EXISTS images (tile_id text NOT NULL PRIMARY KEY, tile_data blob);

CREATE VIEW IF NOT EXISTS tiles AS
    SELECT zoom_level, tile_column, tile_row, tile_data
    FROM map JOIN images ON images.tile_id = map.tile_id;
"#;

const CREATE_INDEX_QUERY: &str =
    "CREATE UNIQUE INDEX IF NOT EXISTS map_index ON map (zoom_level, tile_column, tile_row);";

const INSERT_METADATA_QUERY: &str = "INSERT INTO metadata (name,value) VALUES (?, ?)";
const INSERT_TILE_DATA_QUERY: &str =
    "INSERT OR IGNORE INTO images (tile_id, tile_data) VALUES (?, ?)";
const INSERT_TILE_QUERY: &str =
    "INSERT INTO map (zoom_level, tile_column, tile_row, tile_id) VALUES(?, ?, ?, ?)";

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
        let mut sha1 = Sha1::new();
        sha1.input(png_data);
        let id = sha1.result_str();

        let mut query = conn.prepare_cached(INSERT_TILE_DATA_QUERY)?;
        query.execute(params![id, png_data])?;

        let mut query = conn.prepare_cached(INSERT_TILE_QUERY)?;

        // flip tile Y to match mbtiles spec
        let y = (1u8 << tile_id.zoom) as u32 - 1u32 - tile_id.y;
        query.execute(params![tile_id.zoom, tile_id.x, y, id])?;

        Ok(())
    }

    pub fn close(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.pool.get().unwrap();
        conn.execute(CREATE_INDEX_QUERY, [])?;

        Ok(())
    }
}
