use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

mod storage;
use storage::{query_children, query_sections, Pond, Record};

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

fn exclude_by_name(name: &str, excluded_names: &[&str]) -> bool {
    excluded_names
        .iter()
        .find(|&exname| exname == &name)
        .is_none()
}

fn process_entry(conn: &Connection, path: &PathBuf) -> Result<()> {
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
                'a'..='z' | '-' => Some(ch),
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
        )?;
    } else {
        println!("Skipping {}. No content in {}.", &ordinal, &lang);
    }

    Ok(())
}

fn build_metadata(data: &mut String, record: &Record) -> Result<()> {
    let blob = serde_yaml::to_string(record)?;
    data.push_str(&blob);
    data.push_str("---\n");

    Ok(())
}

/// Builds either a node or a leaf.
fn build_content(conn: &Connection, record: &Record) -> Result<(String, Vec<Record>)> {
    let children = query_children(conn, &record.ordinal)?;

    let data = if children.is_empty() {
        build_leaf(&record)?
    } else {
        build_node(record, &children)?
    };

    Ok((data, children))
}

/// Builds a tree node.
fn build_node(record: &Record, children: &[Record]) -> Result<String> {
    let mut data = String::new();
    build_metadata(&mut data, &record)?;

    data.push_str(&format!("# {}\n\n", &record.title));
    data.push_str(&record.content);
    data.push_str("\n\n## Table of contents\n\n");

    for child in children {
        let item = format!("- [{}](./{}.md)\n", &child.title, &child.slug);
        data.push_str(&item);
    }

    Ok(data)
}

/// Builds a leaf of content.
fn build_leaf(record: &Record) -> Result<String> {
    let mut data = String::new();
    build_metadata(&mut data, &record)?;

    data.push_str(&format!("# {}\n\n", &record.title));
    data.push_str(&record.content);

    Ok(data)
}

/// Takes a record, walks through the dependent tree and writes to a file.
fn write_tree(conn: &Connection, record: &Record, path: &PathBuf) -> Result<()> {
    let (data, children) = build_content(conn, &record)?;

    fs::write(&path.join("index.md"), &data)?;

    for child in children {
        write_node(conn, &child, &path)?;
    }

    Ok(())
}

/// Takes a record, walks through the dependent tree and writes to a file.
fn write_node(conn: &Connection, record: &Record, path: &PathBuf) -> Result<()> {
    let (data, children) = build_content(conn, &record)?;
    let filename = format!("{}.md", record.slug);

    fs::write(&path.join(&filename), &data)?;

    for child in children {
        write_node(conn, &child, &path)?;
    }

    Ok(())
}

fn read_entries<S>(pond: Pond, source: S, excluded_names: &[&str]) -> Result<()>
where
    S: AsRef<Path>,
{
    for entry in fs::read_dir(source)? {
        let entry = entry?.path();
        let name = entry.as_path().file_name().unwrap().to_str().unwrap();
        let conn = pond.get()?;

        if exclude_by_name(name, &excluded_names) {
            process_entry(&conn, &entry)?;
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let source = "../sangaku_manasource/src";
    let target = Path::new("content");
    let excluded_names = vec!["assets", "temario.md"];

    let pond = storage::connect()?;

    fs::create_dir(target)?;
    read_entries(pond.clone(), source, &excluded_names)?;

    let conn = pond.get()?;
    let sections = query_sections(&conn)?;

    for entry in sections {
        let section = target.join(&entry.slug);

        fs::create_dir(&section)?;

        write_tree(&conn, &entry, &section)?;
    }

    Ok(())
}
