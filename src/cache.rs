use anyhow::{anyhow, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, ToSql};
use rusqlite::{Connection, Error::QueryReturnedNoRows, NO_PARAMS};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub type Pond = Pool<SqliteConnectionManager>;

static SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS entry (
  ordinal text NOT NULL PRIMARY KEY,
  parent  text,
  ancestor  NUMBER NOT NULL,
  slug    text NOT NULL,
  title   text NOT NULL,
  difficulty NUMBER,
  content text NOT NULL
);
";

#[derive(Debug)]
pub enum Strategy {
    Memory,
    Disk(PathBuf),
}

impl FromStr for Strategy {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ":memory:" => Ok(Strategy::Memory),
            s => {
                let path = Path::new(s);
                Ok(Strategy::Disk(path.into()))
            }
        }
    }
}

/// Represents record for a cached entry.
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
pub fn connect(strategy: &Strategy) -> Result<Pond> {
    let manager = match strategy {
        Strategy::Disk(path) => SqliteConnectionManager::file(path),
        Strategy::Memory => SqliteConnectionManager::memory(),
    };
    let pool = r2d2::Pool::new(manager)?;
    pool.get()?.execute(SCHEMA, params![])?;

    Ok(pool)
}

/// Executes an arbitrary `select` query against the `entry` table.
///
/// ## Failure
///
/// It fails with a `rusqulite::Error` if the cache is corrupted.
pub fn select_records<P>(conn: &Connection, query: &str, params: P) -> Result<Vec<Record>>
where
    P: IntoIterator,
    P::Item: ToSql,
{
    let mut stmt = conn.prepare(query)?;

    let mut list = Vec::new();
    let rows = stmt.query_map(params, |row| {
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

/// Finds the record for the given ordinal.
///
/// ## Failure
///
/// It fails with a `rusqlite::Error` if the cache is corrupted.
pub fn select_record(conn: &Connection, ordinal: &str) -> Result<Option<Record>> {
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

/// Finds the children entries for the given ordinal.
///
/// ## Failure
///
/// It fails with a `rusqulite::Error` if the cache is corrupted.
pub fn select_children(conn: &Connection, ordinal: &str) -> Result<Vec<Record>> {
    let query = r#"
        SELECT
            *
        FROM
            entry
        WHERE
            parent IS ?
        ORDER BY
            ordinal;
    "#;

    let list = select_records(conn, query, &[ordinal])?;

    Ok(list)
}

/// Finds the section entries. A 'section' is the top level classifier identified by a single
/// digit ordinal and no parent.
///
/// ## Failure
///
/// It fails with a `rusqulite::Error` if the cache is corrupted.
pub fn select_sections(conn: &Connection) -> Result<Vec<Record>> {
    let query = r#"
        SELECT
            *
        FROM
            entry
        WHERE
            parent IS NULL
        ORDER BY
            ordinal;
    "#;
    let list = select_records(conn, query, NO_PARAMS)?;

    Ok(list)
}

/// Finds the sibling entries for the given ordinal.
///
/// ## Failure
///
/// It fails with a `rusqulite::Error` if the cache is corrupted.
pub fn select_siblings(
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

    let prev = select_record(conn, &prev_ordinal)?;
    let next = select_record(conn, &next_ordinal)?;

    Ok((prev, next))
}

pub fn insert_record(conn: &Connection, record: &Record) -> Result<()> {
    let values: [&dyn rusqlite::ToSql; 7] = [
        &record.ordinal,
        &record.parent,
        &record.ancestor,
        &record.slug,
        &record.title,
        &record.difficulty,
        &record.content,
    ];
    conn.execute(
        r#"
        INSERT INTO entry
        (ordinal, parent, ancestor, slug, title, difficulty, content)
        VALUES (?, ?, ?, ?, ?, ?, ?);
    "#,
        &values,
    )?;
    Ok(())
}
