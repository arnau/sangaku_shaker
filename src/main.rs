use anyhow::Result;
use clap::{AppSettings, Clap};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

mod cache;
mod sink;
mod source;
use cache::{select_sections, Strategy};
use sink::write_tree;
use source::read_entries;

fn run<I, O>(input: I, output: O, lang: &str, strategy: Strategy) -> Result<()>
where
    I: AsRef<Path>,
    O: AsRef<Path>,
{
    let excluded_names = vec!["assets", "temario.md"];

    // let source = "../sangaku_manasource/src";
    // let target = Path::new("content");
    // let lang = "en";
    // let strategy = Strategy::Memory;
    // let storage = Strategy::Disk(Path::new("test.db").to_path_buf());

    let pond = cache::connect(&strategy)?;

    // Sourcing phase
    read_entries(pond.clone(), input, &excluded_names, lang)?;

    let conn = pond.get()?;
    let sections = select_sections(&conn)?;

    // Sinking phase
    fs::create_dir(&output)?;
    for entry in sections {
        let section = output.as_ref().join(&entry.slug);

        fs::create_dir(&section)?;

        write_tree(&conn, &entry, &section)?;
    }

    // sections
    //     .into_iter()
    //     .map(|entry| {
    //         let pond = pond.clone();
    //         let section = target.join(&entry.slug);
    //         fs::create_dir(&section).unwrap();

    //         thread::spawn(move || {
    //             let conn = pond.get().unwrap();
    //             write_tree(&conn, &entry, &section).unwrap();
    //         })
    //     })
    //     .collect::<Vec<_>>()
    //     .into_iter()
    //     .map(thread::JoinHandle::join)
    //     .collect::<std::result::Result<(), _>>()
    //     .unwrap();

    Ok(())
}

#[derive(Debug, Clap)]
#[clap(name = "shaker", version, global_setting(AppSettings::ColoredHelp))]
struct Cli {
    /// Cache strategy
    ///
    /// If a file path is provided it attempts to create a SQLite database or reuse it if it
    /// already exists.
    #[clap(long, short = 'c', value_name = "path", default_value = ":memory:")]
    cache_path: Strategy,
    /// Input directory. Expects a valid mana source
    #[clap(long, short = 'i', value_name = "path")]
    input_path: PathBuf,
    /// Output directory
    #[clap(long, short = 'o', value_name = "path")]
    output_path: PathBuf,
    /// Output language
    #[clap(long, value_name = "code", default_value = "en", possible_values = &["en", "ca", "es"])]
    lang: String,
}

fn main() {
    let cli: Cli = Cli::parse();
    match run(cli.input_path, cli.output_path, &cli.lang, cli.cache_path) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{:?}", err);
        }
    };
}
