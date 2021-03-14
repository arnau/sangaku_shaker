use anyhow::Result;
use rusqlite::NO_PARAMS;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

mod storage;

#[derive(Debug, Serialize, Deserialize)]
struct MetaItem {
    pub lang: String,
    pub name: String,
    pub desc: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    pub number: String,
    pub parent: Option<String>,
    pub difficulty: Option<u32>,
    #[serde(default)]
    pub data: Vec<MetaItem>,
    #[serde(default)]
    pub names: Vec<MetaItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    ordinal: String,
    parent: Option<String>,
    ancestor: u32,
    slug: String,
    title: String,
    difficulty: Option<u32>,
    content: String,
}

fn exclude_by_name(name: &str, excluded_names: &[&str]) -> bool {
    excluded_names
        .iter()
        .find(|&exname| exname == &name)
        .is_none()
}

fn process_entry(pond: storage::Pond, path: &PathBuf) -> Result<()> {
    let lang = "en";
    let mut handle = fs::File::open(path.join("metadata.json"))?;
    let mut data = String::new();
    handle.read_to_string(&mut data)?;

    let meta: Metadata = serde_json::from_str(&data)?;
    let item = if let Some(data) = meta.data.iter().find(|item| item.lang == lang) {
        Some(data)
    } else {
        meta.names.iter().find(|item| item.lang == lang)
    };

    let ordinal = meta.number;

    if let Some(item) = item {
        let parent = meta.parent;
        let trail: Vec<&str> = ordinal.split('.').collect();
        let ancestor: u32 = trail[0].parse()?;
        let slug: String = item
            .name
            .to_lowercase()
            .chars()
            .filter_map(|ch| match ch {
                'a'...'z' => Some(ch),
                '-' => Some(ch),
                ' ' => Some('-'),
                _ => None,
            })
            .collect();
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

        let conn = pond.get()?;
        let values: [&dyn rusqlite::ToSql; 7] = [
            &ordinal,
            &parent,
            &ancestor,
            &slug,
            &title,
            &difficulty,
            &content,
        ];
        conn.execute(
            r#"
        INSERT INTO entry
        (ordinal, parent, ancestor, slug, title, difficulty, content)
        VALUES (?, ?, ?, ?, ?, ?, ?);
    "#,
            &values,
        )
        .unwrap_or_else(|err| {
            dbg!(&ordinal);
            0
        });
    } else {
        println!("Skipping {}. No content in {}.", &ordinal, &lang);
    }

    Ok(())
}

fn main() -> Result<()> {
    let source = "../sangaku_manasource/src";
    let target = "content";
    let pond = storage::connect()?;

    // Create target dir
    // fs::create_dir(target)?;

    // Flesh out target structure
    let excluded_names = vec!["assets", "temario.md"];
    let mut entries = fs::read_dir(source)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort();

    for entry in entries.iter() {
        let name = entry.as_path().file_name().unwrap().to_str().unwrap();
        let pond = pond.clone();

        if exclude_by_name(name, &excluded_names) {
            process_entry(pond, entry)?;
        }
    }

    let conn = pond.get()?;
    let mut stmt = conn.prepare("SELECT * FROM entry ORDER BY ordinal;")?;
    // let mut list = Vec::new();
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
        let entry = result?;
        // list.push(result?);
        dbg!(&entry);
    }

    Ok(())
}
