use std::{
    fmt::format,
    fs::File,
    io::{self, BufRead, BufReader},
};

use clap::{App, Arg};
use regex::{Regex, RegexBuilder};
use walkdir::{DirEntry, WalkDir};

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct Config {
    pattern: Regex,
    files: Vec<String>,
    recursive: bool,
    count: bool,
    invert_match: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("grepr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust grep")
        .arg(
            Arg::with_name("pattern")
                .value_name("PATTERN")
                .required(true)
                .help("Search pattern"),
        )
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .multiple(true)
                .default_value("-")
                .help("Input file(s)"),
        )
        .arg(
            Arg::with_name("recursive")
                .short("r")
                .long("recursive")
                .takes_value(false)
                .help("Recursive search"),
        )
        .arg(
            Arg::with_name("count")
                .short("c")
                .long("count")
                .takes_value(false)
                .help("Count occurrences"),
        )
        .arg(
            Arg::with_name("invert")
                .short("v")
                .long("invert-match")
                .takes_value(false)
                .help("Invert match"),
        )
        .arg(
            Arg::with_name("insensitive")
                .short("i")
                .long("insensitive")
                .takes_value(false)
                .help("Case-insensitive"),
        )
        .get_matches();

    let pattern = matches.value_of("pattern").unwrap();
    let pattern = RegexBuilder::new(pattern)
        .case_insensitive(matches.is_present("insensitive"))
        .build()
        .map_err(|_| format!("Invalid pattern \"{}\"", pattern))?;

    Ok(Config {
        pattern,
        files: matches.values_of_lossy("files").unwrap(),
        recursive: matches.is_present("recursive"),
        count: matches.is_present("count"),
        invert_match: matches.is_present("invert"),
    })
}

fn find_files(paths: &[String], recursive: bool) -> Vec<MyResult<String>> {
    let mut results = vec![];
    for path in paths {
        if path == "-" {
            results.push(Ok(String::from(path)));
            continue;
        }

        let mut files = WalkDir::new(path).into_iter().peekable();
        let root = files.peek().unwrap();

        if !recursive && root.as_ref().map(|e| e.path().is_dir()).unwrap_or(false) {
            results.push(Err(From::from(format!("{} is a directory", path))));
        } else {
            results.extend(
                files
                    .filter(|res| res.is_err() || !res.as_ref().unwrap().path().is_dir())
                    .map(|r| {
                        r.map_err(|e| {
                            From::from(format!(
                                "{}: {}",
                                e.path()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or("error".to_string()),
                                e.io_error()
                                    .map(|e| e.to_string())
                                    .unwrap_or("unknown error".to_string())
                            ))
                        })
                        .map(|entry| entry.path().display().to_string())
                    }),
            );
        }
    }
    results
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn find_lines<T: BufRead>(
    mut file: T,
    pattern: &Regex,
    invert_match: bool,
) -> MyResult<Vec<String>> {
    let mut results = vec![];
    loop {
        let mut buf = String::new();
        file.read_line(&mut buf)?;
        if buf.len() == 0 {
            break;
        }

        match pattern.is_match(&buf) {
            true if !invert_match => results.push(buf),
            false if invert_match => results.push(buf),
            _ => (),
        }
    }
    Ok(results)
}

pub fn run(config: Config) -> MyResult<()> {
    let entries = find_files(&config.files, config.recursive);
    for entry in &entries {
        match entry {
            Err(e) => eprintln!("{}", e),
            Ok(filename) => match open(&filename) {
                Err(e) => eprintln!("{}: {}", filename, e),
                Ok(file) => {
                    let prefix = if entries.len() > 1 {
                        format!("{}:", &filename)
                    } else {
                        "".to_string()
                    };
                    let matches = find_lines(file, &config.pattern, config.invert_match)?;
                    if config.count {
                        println!("{}{}", prefix, matches.len());
                    } else {
                        for m in matches {
                            print!("{}{}", prefix, m);
                        }
                    }
                }
            },
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{find_files, find_lines};
    use rand::{distributions::Alphanumeric, Rng};
    use regex::{Regex, RegexBuilder};
    use std::io::Cursor;

    #[test]
    fn test_find_files() {
        // Verify that the function finds a file known to exist
        let files = find_files(&["./tests/inputs/fox.txt".to_string()], false);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].as_ref().unwrap(), "./tests/inputs/fox.txt");

        // The function should reject a directory without the recursive option
        let files = find_files(&["./tests/inputs".to_string()], false);
        assert_eq!(files.len(), 1);
        if let Err(e) = &files[0] {
            assert_eq!(e.to_string(), "./tests/inputs is a directory");
        }

        // Verify the function recurses to find four files in the directory
        let res = find_files(&["./tests/inputs".to_string()], true);
        let mut files: Vec<String> = res
            .iter()
            .map(|r| r.as_ref().unwrap().replace("\\", "/"))
            .collect();
        files.sort();
        assert_eq!(files.len(), 4);
        assert_eq!(
            files,
            vec![
                "./tests/inputs/bustle.txt",
                "./tests/inputs/empty.txt",
                "./tests/inputs/fox.txt",
                "./tests/inputs/nobody.txt",
            ]
        );

        // Generate a random string to represent a nonexistent file
        let bad: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();

        // Verify that the function returns the bad file as an error
        let files = find_files(&[bad], false);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_err());
    }

    #[test]
    fn test_find_lines() {
        let text = b"Lorem\nIpsum\r\nDOLOR";

        // The pattern _or_ should match the one line, "Lorem"
        let re1 = Regex::new("or").unwrap();
        let matches = find_lines(Cursor::new(&text), &re1, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);

        // When inverted, the function should match the other two lines
        let matches = find_lines(Cursor::new(&text), &re1, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // This regex will be case-insensitive
        let re2 = RegexBuilder::new("or")
            .case_insensitive(true)
            .build()
            .unwrap();

        // The two lines "Lorem" and "DOLOR" should match
        let matches = find_lines(Cursor::new(&text), &re2, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // When inverted, the one remaining line should match
        let matches = find_lines(Cursor::new(&text), &re2, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);
    }
}
