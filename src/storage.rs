use anyhow::{anyhow, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use rusqlite::{Connection, Error::QueryReturnedNoRows, NO_PARAMS};
use serde::{Deserialize, Serialize};
use std::include_str;

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

/// Finds the sibling entries for the given ordinal.
pub fn query_siblings(
    conn: &Connection,
    ordinal: &str,
) -> Result<(Option<Record>, Option<Record>)> {
    let trail: Vec<&str> = ordinal.split('.').collect();
    let upbound = trail.len() - 1;
    let current_index: u32 = trail[upbound].parse()?;
    let prev_index = current_index - 1;
    let next_index = current_index + 1;

    let prev_ordinal = format!("{}.{}", trail[0..upbound].join("."), prev_index);
    let next_ordinal = format!("{}.{}", trail[0..upbound].join("."), next_index);

    let prev = query_record(conn, &prev_ordinal)?;
    let next = query_record(conn, &next_ordinal)?;

    Ok((prev, next))
}

pub fn query_record(conn: &Connection, ordinal: &str) -> Result<Option<Record>> {
    let result = conn.query_row(
        r#"
        SELECT
            *
        FROM
            entry
        WHERE
            ordinal = ?
        "#,
        &[ordinal],
        |row| {
            Ok(Record {
                ordinal: row.get(0)?,
                parent: row.get(1)?,
                ancestor: row.get(2)?,
                slug: row.get(3)?,
                title: row.get(4)?,
                difficulty: row.get(5)?,
                content: row.get(6)?,
            })
        },
    );

    match result {
        Err(QueryReturnedNoRows) => Ok(None),
        Ok(record) => Ok(Some(record)),
        Err(err) => Err(anyhow!(err)),
    }
}
