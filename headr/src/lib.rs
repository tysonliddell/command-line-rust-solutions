use clap::{App, Arg};
use std::cmp::min;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: usize,
    bytes: Option<usize>,
}

fn parse_positive_int(val: &str) -> MyResult<usize> {
    // these doesn't quite work. Doesn't error on x == 0
    // val.parse::<usize>().map_err(|_| From::from(val))
    // or more concisely
    // val.parse().map_err(|_| From::from(val))

    match val.parse() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(From::from(val)),
    }
}

#[test]
fn test_parse_positive_int() {
    // 3 is an OK integer
    let res = parse_positive_int("3");
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 3);

    // Any string is an error
    let res = parse_positive_int("foo");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "foo".to_string());

    // A zero is an error
    let res = parse_positive_int("0");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "0".to_string())
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("headr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust head")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .multiple(true)
                .default_value("-")
                .help("Input file(s)"),
        )
        .arg(
            Arg::with_name("lines")
                .short("n")
                .long("lines")
                .value_name("LINES")
                .default_value("10")
                .help("Number of lines"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .value_name("BYTES")
                .help("Number of bytes")
                .conflicts_with("lines"),
        )
        .get_matches();

    let lines = matches
        .value_of("lines")
        .map(parse_positive_int)
        .transpose()
        .map_err(|e| format!("illegal line count -- {}", e))?;

    let bytes = matches
        .value_of("bytes")
        .map(parse_positive_int)
        .transpose()
        .map_err(|e| format!("illegal byte count -- {}", e))?;

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        lines: lines.unwrap(),
        bytes,
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn print_lines(mut file: Box<dyn BufRead>, mut num_lines: usize) {
    let mut buf = String::new();
    while num_lines > 0 && file.read_line(&mut buf).unwrap() != 0 {
        print!("{}", buf);
        buf.clear();
        num_lines -= 1;
    }
    io::stdout().flush().unwrap();
}

fn print_bytes(mut file: Box<dyn BufRead>, mut num_bytes: usize) {
    while num_bytes > 0 {
        let bytes = file.fill_buf().unwrap();
        if bytes.len() == 0 {
            // no data left in the file
            break;
        }
        print!(
            "{}",
            String::from_utf8_lossy(&bytes[..min(bytes.len(), num_bytes)])
        );

        let length = bytes.len();
        file.consume(length);
        num_bytes -= min(length, num_bytes);
    }
    io::stdout().flush().unwrap();
}

pub fn run(config: Config) -> MyResult<()> {
    let mut file_succeeded = false;
    let print_header = config.files.len() > 1;

    for filename in &config.files {
        match open(filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(file) => {
                if print_header {
                    println!(
                        "{}==> {} <==",
                        if file_succeeded { "\n" } else { "" },
                        filename
                    );
                }
                if let Some(num_bytes) = config.bytes {
                    print_bytes(file, num_bytes);
                } else {
                    print_lines(file, config.lines);
                }
                file_succeeded = true;
            }
        }
    }
    Ok(())
}
