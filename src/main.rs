use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

mod cache;
mod source;
use cache::{select_children, select_sections, select_siblings, Record};
use source::read_entries;

fn build_metadata(data: &mut String, record: &Record) -> Result<()> {
    let blob = serde_yaml::to_string(record)?;
    data.push_str(&blob);
    data.push_str("---\n");

    Ok(())
}

/// Builds either a node or a leaf.
fn build_content(conn: &Connection, record: &Record) -> Result<(String, Vec<Record>)> {
    let children = select_children(conn, &record.ordinal)?;

    let data = if children.is_empty() {
        let siblings = select_siblings(&conn, &record.ordinal)?;

        build_leaf(&record, siblings)?
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
    data.push_str("\n\n");
    data.push_str("## Table of contents\n\n");

    for child in children {
        let item = format!("- [{}](./{}.md)\n", &child.title, &child.slug);
        data.push_str(&item);
    }

    Ok(data)
}

/// Builds a leaf of content.
fn build_leaf(record: &Record, siblings: (Option<Record>, Option<Record>)) -> Result<String> {
    let (prev, next) = siblings;
    let mut nav = Vec::new();
    let mut data = String::new();
    build_metadata(&mut data, &record)?;

    if let Some(prev) = prev {
        nav.push(format!("- Previous: [{}]({}.md)", &prev.title, &prev.slug));
    }

    if let Some(next) = next {
        nav.push(format!("- Next: [{}]({}.md)", &next.title, &next.slug));
    }

    data.push_str(&format!("# {}\n\n", &record.title));
    data.push_str(&record.content);
    data.push_str("\n\n");
    data.push_str("## Navigation\n\n");
    data.push_str(&(nav.join("\n")));

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

fn main() -> Result<()> {
    let source = "../sangaku_manasource/src";
    let target = Path::new("content");
    let excluded_names = vec!["assets", "temario.md"];
    let lang = "en";

    let pond = cache::connect()?;

    fs::create_dir(target)?;
    read_entries(pond.clone(), source, &excluded_names, lang)?;

    let conn = pond.get()?;
    let sections = select_sections(&conn)?;

    for entry in sections {
        let section = target.join(&entry.slug);

        fs::create_dir(&section)?;

        write_tree(&conn, &entry, &section)?;
    }

    Ok(())
}
