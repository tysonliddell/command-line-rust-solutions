use crate::Line::*;
use clap::{App, Arg};
use std::{
    cmp::Ordering,
    fs::File,
    io::{self, BufRead, BufReader, Lines},
    iter::Peekable,
};

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct Config {
    file1: String,
    file2: String,
    show_col1: bool,
    show_col2: bool,
    show_col3: bool,
    insensitive: bool,
    delimiter: String,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("commr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust comm")
        .arg(
            Arg::with_name("file1")
                .value_name("FILE1")
                .required(true)
                .help("Input file 1"),
        )
        .arg(
            Arg::with_name("file2")
                .value_name("FILE2")
                .required(true)
                .help("Input file 2"),
        )
        .arg(
            Arg::with_name("suppress1")
                .short("1")
                .help("Suppress printing of column 1"),
        )
        .arg(
            Arg::with_name("suppress2")
                .short("2")
                .help("Suppress printing of column 2"),
        )
        .arg(
            Arg::with_name("suppress3")
                .short("3")
                .help("Suppress printing of column 3"),
        )
        .arg(
            Arg::with_name("insensitive")
                .short("i")
                .help("Case-insensitive comparison of lines"),
        )
        .arg(
            Arg::with_name("delimiter")
                .value_name("DELIM")
                .short("d")
                .long("output-delimiter")
                .takes_value(true)
                .default_value("\t")
                .hide_default_value(true)
                .help("Output delimiter (defaults to TAB)"),
        )
        .get_matches();

    Ok(Config {
        file1: matches.value_of("file1").unwrap().to_string(),
        file2: matches.value_of("file2").unwrap().to_string(),
        show_col1: !matches.is_present("suppress1"),
        show_col2: !matches.is_present("suppress2"),
        show_col3: !matches.is_present("suppress3"),
        insensitive: matches.is_present("insensitive"),
        delimiter: matches.value_of("delimiter").unwrap().to_string(),
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(
            File::open(filename).map_err(|e| format!("{}: {}", filename, e))?,
        ))),
    }
}

enum Line {
    File1(String),
    File2(String),
    Both(String),
}

fn get_next_line(
    file1_lines: &mut Peekable<impl Iterator<Item=String>>,
    file2_lines: &mut Peekable<impl Iterator<Item=String>>,
    insensitive: bool,
) -> Option<Line> {
    let line1 = file1_lines.peek().cloned();
    let line2 = file2_lines.peek().cloned();

    if line1.is_none() && line2.is_none() {
        return None;
    } else if line1.is_none() {
        file2_lines.next();
        return Some(File2(line2.unwrap()));
    } else if line2.is_none() {
        file1_lines.next();
        return Some(File1(line1.unwrap()));
    }

    let line1 = line1.unwrap();
    let line2 = line2.unwrap();

    let res = if insensitive {
        line1.to_lowercase().cmp(&line2.to_lowercase())
    } else {
        line1.cmp(&line2)
    };
    match res {
        Ordering::Equal => {
            file1_lines.next();
            file2_lines.next();
            Some(Both(line1))
        }
        Ordering::Less => {
            file1_lines.next();
            Some(File1(line1))
        }
        Ordering::Greater => {
            file2_lines.next();
            Some(File2(line2))
        }
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let file1 = &config.file1;
    let file2 = &config.file2;
    if file1 == "-" && file2 == "-" {
        return Err(From::from("Both input files cannot be STDIN (\"-\")"));
    }

    let get_delims = |line: &Line| match line {
        File2(_) if config.show_col1 => config.delimiter.clone(),
        Both(_) if config.show_col1 && config.show_col2 => config.delimiter.repeat(2),
        Both(_) if config.show_col1 || config.show_col2 => config.delimiter.clone(),
        _ => "".to_string(),
    };

    let mut file1_lines = open(file1)?.lines().filter_map(Result::ok).peekable();
    let mut file2_lines = open(file2)?.lines().filter_map(Result::ok).peekable();

    loop {
        let line = get_next_line(&mut file1_lines, &mut file2_lines, config.insensitive);
        if line.is_none() {
            break;
        }
        let line = line.unwrap();

        match &line {
            File1(line_text) if config.show_col1 => {
                println!("{}", line_text)
            }
            File2(line_text) if config.show_col2 => {
                println!("{}{}", get_delims(&line), line_text)
            }
            Both(line_text) if config.show_col3 => {
                println!("{}{}", get_delims(&line), line_text)
            }
            _ => (),
        }
    }

    Ok(())
}
