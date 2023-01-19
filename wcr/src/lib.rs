use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

use clap::{App, Arg};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    bytes: bool,
    lines: bool,
    characters: bool,
    words: bool,
}

#[derive(Debug, PartialEq)]
pub struct FileInfo {
    num_lines: usize,
    num_words: usize,
    num_bytes: usize,
    num_chars: usize,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("wcr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust wc")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .multiple(true)
                .default_value("-")
                .help("Input file(s)"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .help("Include the byte counts")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("lines")
                .short("l")
                .long("lines")
                .help("Include the line counts")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("chars")
                .short("m")
                .long("chars")
                .help("Include the character counts")
                .takes_value(false)
                .conflicts_with("bytes"),
        )
        .arg(
            Arg::with_name("words")
                .short("w")
                .long("words")
                .help("Include the word counts")
                .takes_value(false),
        )
        .get_matches();

    let mut bytes = matches.is_present("bytes");
    let mut lines = matches.is_present("lines");
    let mut words = matches.is_present("words");
    let characters = matches.is_present("chars");

    // check user didn't provide any options:
    if [bytes, lines, characters, words].iter().all(|v| !v) {
        bytes = true;
        lines = true;
        words = true;
    }

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        bytes,
        lines,
        characters,
        words,
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn count(mut file: impl BufRead) -> MyResult<FileInfo> {
    let mut num_lines = 0;
    let mut num_words = 0;
    let mut num_bytes = 0;
    let mut num_chars = 0;

    let mut buf = String::new();
    num_bytes += file.read_line(&mut buf)?;
    while buf.len() > 0 {
        num_lines += 1;
        num_words += buf.split_ascii_whitespace().collect::<Vec<_>>().len();
        num_chars += buf.chars().collect::<Vec<_>>().len();

        buf.clear();
        num_bytes += file.read_line(&mut buf)?;
    }

    Ok(FileInfo {
        num_lines,
        num_words,
        num_bytes,
        num_chars,
    })
}

fn print_info_line(config: &Config, info: &FileInfo, line_desc: &str) {
    if config.lines {
        print!("{:>8}", info.num_lines);
    }
    if config.words {
        print!("{:>8}", info.num_words);
    }
    if config.bytes {
        print!("{:>8}", info.num_bytes);
    }
    if config.characters {
        print!("{:>8}", info.num_chars);
    }

    if line_desc != "-" {
        println!(" {}", line_desc);
    } else {
        println!();
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let mut total_counts = FileInfo {
        num_bytes: 0,
        num_chars: 0,
        num_lines: 0,
        num_words: 0,
    };
    for filename in &config.files {
        match open(&filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(file) => {
                let info = count(file)?;
                print_info_line(&config, &info, &filename);

                total_counts.num_bytes += info.num_bytes;
                total_counts.num_chars += info.num_chars;
                total_counts.num_lines += info.num_lines;
                total_counts.num_words += info.num_words;
            }
        }
    }
    if config.files.len() > 1 {
        print_info_line(&config, &total_counts, "total");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{count, FileInfo};
    use std::io::Cursor;

    #[test]
    fn test_count() {
        let text = "I don't want the world. I just want your half.\r\n";
        let info = count(Cursor::new(text));
        assert!(info.is_ok());
        let expected = FileInfo {
            num_lines: 1,
            num_words: 10,
            num_chars: 48,
            num_bytes: 48,
        };
        assert_eq!(info.unwrap(), expected);
    }
}
