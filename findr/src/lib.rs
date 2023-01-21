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
    })
}

fn get_entry_type(entry: &DirEntry) -> EntryType {
    if entry.file_type().is_dir() {
        return Dir;
    } else if entry.file_type().is_file() {
        return File;
    } else if entry.file_type().is_symlink() {
        return Link;
    } else {
        unreachable!("File type error");
    }
}

pub fn run(config: Config) -> MyResult<()> {
    for path in config.paths {
        for entry in WalkDir::new(path) {
            match entry {
                Err(e) => eprintln!("{}", e),
                Ok(entry) => {
                    if !config.entry_types.contains(&get_entry_type(&entry)) {
                        continue;
                    }

                    let filename = entry.file_name().to_str().ok_or("error reading filename")?;
                    if config.names.is_empty() || config.names.iter().any(|r| r.is_match(filename))
                    {
                        println!("{}", entry.path().display())
                    }
                }
            }
        }
    }
    Ok(())
}
