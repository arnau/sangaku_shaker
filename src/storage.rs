use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use rusqlite::{Connection, Transaction, NO_PARAMS};
use serde::{Deserialize, Serialize};
use std::include_str;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;

pub type Pond = Pool<SqliteConnectionManager>;

/// A record for a content entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub ordinal: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip)]
    pub ancestor: u32,
    pub slug: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<u32>,
    #[serde(skip)]
    pub content: String,
}

/// Creates a pool for an in-memory database.
pub fn connect() -> Result<Pond> {
    // let manager = SqliteConnectionManager::file("test.db");
    let manager = SqliteConnectionManager::memory();
    let pool = r2d2::Pool::new(manager)?;
    let bootstrap = include_str!("./schema.sql");
    pool.get()?.execute(&bootstrap, params![])?;

    Ok(pool)
}

pub fn query_children(conn: &Connection, ordinal: &str) -> Result<Vec<Record>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            *
        FROM
            entry
        WHERE
            parent IS ?
        ORDER BY
            ordinal;
    "#,
    )?;

    let mut list = Vec::new();
    let rows = stmt.query_map(&[ordinal], |row| {
        Ok(Record {
            ordinal: row.get(0)?,
            parent: row.get(1)?,
            ancestor: row.get(2)?,
            slug: row.get(3)?,
            title: row.get(4)?,
            difficulty: row.get(5)?,
            content: row.get(6)?,
        })
    })?;

    for result in rows {
        list.push(result?);
    }

    Ok(list)
}

pub fn query_sections(conn: &Connection) -> Result<Vec<Record>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            *
        FROM
            entry
        WHERE
            parent IS NULL
        ORDER BY
            ordinal;
    "#,
    )?;
    let mut list = Vec::new();

    let rows = stmt.query_map(NO_PARAMS, |row| {
        Ok(Record {
            ordinal: row.get(0)?,
            parent: row.get(1)?,
            ancestor: row.get(2)?,
            slug: row.get(3)?,
            title: row.get(4)?,
            difficulty: row.get(5)?,
            content: row.get(6)?,
        })
    })?;

    for result in rows {
        list.push(result?);
    }

    Ok(list)
}
