//! Helpers to source from the upstream mana structure.
//!
//! The expected directory structure is:
//!
//! ```nocode
//! .
//! └── src
//!    ├── 1
//!    │  └── metadata.json
//!    ├── 1.1
//!    │  └── metadata.json
//!    ├── 1.1.2
//!    │  ├── exercises
//!    │  │  ├── ca.md
//!    │  │  ├── en.md
//!    │  │  └── es.md
//!    │  ├── metadata.json
//!    │  └── theory
//!    │     ├── ca.md
//!    │     ├── en.md
//!    │     └── es.md
//!    ├── 1.1.3
//!    .
//!    .
//!    .
//!    └── assets
//! ```

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use super::cache::{insert_record, Pond, Record};

#[derive(Debug, Serialize, Deserialize)]
struct MetaItem {
    pub lang: String,
    pub name: String,
    pub desc: Option<String>,
}

/// Represents the metadata structure held in `metadata.json`.
#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    pub number: String,
    pub parent: Option<String>,
    pub difficulty: Option<u32>,
    #[serde(default)]
    pub data: Vec<MetaItem>,
}

fn exclude_by_name(name: &str, excluded_names: &[&str]) -> bool {
    excluded_names
        .iter()
        .find(|&exname| exname == &name)
        .is_none()
}

/// Tranforms the given string into its equivalent with ASCII lowercase `a..z` and `-` instead of
/// spaces.
fn slug(input: &str) -> String {
    input
        .to_ascii_lowercase()
        .to_lowercase()
        .chars()
        .filter_map(|ch| match ch {
            'a'..='z' | '-' => Some(ch),
            ' ' => Some('-'),
            _ => None,
        })
        .collect()
}

fn process_entry(conn: &Connection, path: &PathBuf, lang: &str) -> Result<()> {
    let mut handle = fs::File::open(path.join("metadata.json"))?;
    let mut data = String::new();
    handle.read_to_string(&mut data)?;

    let meta: Metadata = serde_json::from_str(&data)?;
    let item = meta.data.iter().find(|item| item.lang == lang);

    let ordinal = meta.number;

    if let Some(item) = item {
        let parent = meta.parent;
        let trail: Vec<&str> = ordinal.split('.').collect();
        let ancestor: u32 = trail[0].parse()?;
        let slug: String = slug(&item.name);
        let title = item.name.clone();
        let difficulty = meta.difficulty;
        let content = if let Some(desc) = item.desc.clone() {
            desc
        } else {
            let content_path = format!("theory/{}.md", lang);
            let mut handle = fs::File::open(path.join(&content_path))?;
            let mut data = String::new();
            handle.read_to_string(&mut data)?;

            data
        };
        let record = Record {
            ordinal,
            parent,
            ancestor,
            slug,
            title,
            difficulty,
            content,
        };

        insert_record(conn, &record)?;
    } else {
        println!("Skipping {}. No content for {}.", &ordinal, &lang);
    }

    Ok(())
}

/// Reads the section directories from the given mana path and processes every entry found in them
/// for the given language.
pub fn read_entries<S>(pond: Pond, source: S, excluded_names: &[&str], lang: &str) -> Result<()>
where
    S: AsRef<Path>,
{
    for entry in fs::read_dir(source)? {
        let entry = entry?.path();
        let name = entry.as_path().file_name().unwrap().to_str().unwrap();
        let conn = pond.get()?;

        if exclude_by_name(name, &excluded_names) {
            process_entry(&conn, &entry, lang)?;
        }
    }

    Ok(())
}
