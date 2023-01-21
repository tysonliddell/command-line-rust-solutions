use crate::EntryType::*;
use clap::{App, Arg};
use regex::Regex;
use walkdir::{DirEntry, WalkDir};

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Eq, PartialEq)]
enum EntryType {
    Dir,
    File,
    Link,
}

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    names: Vec<Regex>,
    entry_types: Vec<EntryType>,
    max_depth: Option<usize>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("findr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust find")
        .arg(
            Arg::with_name("names")
                .short("n")
                .long("name")
                .takes_value(true)
                .value_name("NAME")
                .multiple(true)
                .help("Name"),
        )
        .arg(
            Arg::with_name("types")
                .short("t")
                .long("type")
                .takes_value(true)
                .value_name("TYPE")
                .multiple(true)
                .possible_values(&["f", "d", "l"])
                .help("Entry type"),
        )
        .arg(
            Arg::with_name("paths")
                .value_name("PATH")
                .multiple(true)
                .default_value(".")
                .help("Search paths"),
        )
        .arg(
            Arg::with_name("max_depth")
                .long("maxdepth")
                .value_name("MAX_DEPTH")
                .required(false)
                .help("Maximum search depth")
        )
        .get_matches();

    Ok(Config {
        paths: matches.values_of_lossy("paths").unwrap(),
        names: matches
            .values_of("names")
            .unwrap_or_default()
            .map(|v| Regex::new(v).map_err(|_| format!("Invalid --name \"{}\"", v)))
            .collect::<Result<_, _>>()?,
        entry_types: matches
            .values_of("types")
            .map(|values| {
                values
                    .map(|v| match v {
                        "f" => File,
                        "d" => Dir,
                        "l" => Link,
                        _ => unreachable!("Invalid type"),
                    })
                    .collect()
            })
            .unwrap_or(vec![File, Dir, Link]),
        max_depth: matches.value_of("max_depth").map(|v| v.parse()).transpose()?,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let type_filter = |entry: &DirEntry| {
        config.entry_types.is_empty()
            || config.entry_types.iter().any(|e| match e {
                File => entry.file_type().is_file(),
                Dir => entry.file_type().is_dir(),
                Link => entry.file_type().is_symlink(),
            })
    };

    let name_filter = |entry: &DirEntry| {
        config.names.is_empty()
            || config
                .names
                .iter()
                .any(|re| re.is_match(&entry.file_name().to_string_lossy()))
    };

    let max_depth_filter = |entry: &DirEntry| {
        config.max_depth.is_none() || entry.depth() <= config.max_depth.unwrap()
    };

    for path in config.paths {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| match e {
                Err(e) => {
                    eprintln!("{}", e);
                    None
                }
                Ok(e) => Some(e),
            })
            .filter(type_filter)
            .filter(name_filter)
            .filter(max_depth_filter)
            .for_each(|e| println!("{}", e.path().display()));
    }
    Ok(())
}
