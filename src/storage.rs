use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use rusqlite::{Connection, Transaction, NO_PARAMS};
use std::include_str;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;

pub type Pond = Pool<SqliteConnectionManager>;

/// Creates a pool for an in-memory database.
pub fn connect() -> Result<Pond> {
    // let manager = SqliteConnectionManager::file("test.db");
    let manager = SqliteConnectionManager::memory();
    let pool = r2d2::Pool::new(manager)?;
    let bootstrap = include_str!("./schema.sql");
    pool.get()?.execute(&bootstrap, params![])?;

    Ok(pool)
}
