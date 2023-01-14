use clap::{App, Arg};
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    number_lines: bool,
    number_nonblank_lines: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("catr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust cat")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .multiple(true)
                .default_value("-"),
        )
        .arg(
            Arg::with_name("number")
                .short("n")
                .long("number")
                .help("Number lines")
                .takes_value(false)
                .conflicts_with("number_nonblank"),
        )
        .arg(
            Arg::with_name("number_nonblank")
                .short("b")
                .long("number-nonblank")
                .help("Number nonblank lines")
                .takes_value(false),
        )
        .get_matches();

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        number_lines: matches.is_present("number"),
        number_nonblank_lines: matches.is_present("number_nonblank"),
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn print_file(file: Box<dyn BufRead>, config: &Config) -> MyResult<()> {
    let mut blank_count = 0;
    for (line_num, line) in file.lines().enumerate() {
        let line = line?;
        if line.is_empty() {
            blank_count += 1;
        }

        let prefix = if config.number_lines {
            format!("{:>6}\t", line_num + 1)
        } else if config.number_nonblank_lines && !line.is_empty() {
            format!("{:>6}\t", line_num + 1 - blank_count)
        } else {
            String::from("")
        };

        println!("{}{}", prefix, line);
    }
    Ok(())
}

pub fn run(config: Config) -> MyResult<()> {
    for filename in &config.files {
        match open(&filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(file) => print_file(file, &config)?,
        }
    }
    Ok(())
}
