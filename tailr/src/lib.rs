use std::{
    cmp::{max},
    fs::{metadata, File},
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
};

use clap::{App, Arg};

use crate::TakeValue::*;

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: TakeValue,
    bytes: Option<TakeValue>,
    quiet: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TakeValue {
    PlusZero,
    TakeNum(i64),
}

fn parse_num(value: &str) -> MyResult<TakeValue> {
    if value == "+0" {
        return Ok(PlusZero);
    }
    if value.starts_with('+') {
        Ok(TakeNum(value.parse().map_err(|_| value)?))
    } else {
        let value = value.parse().map_err(|_| value)?;
        Ok(TakeNum(if value <= 0 { value } else { -value }))
    }
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("tailr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust tail")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .required(true)
                .multiple(true)
                .help("Input file(s)"),
        )
        .arg(
            Arg::with_name("lines")
                .value_name("LINES")
                .short("n")
                .long("lines")
                .allow_hyphen_values(true)
                .default_value("10")
                .help("Number of lines"),
        )
        .arg(
            Arg::with_name("bytes")
                .value_name("BYTES")
                .short("c")
                .long("bytes")
                .allow_hyphen_values(true)
                .conflicts_with("lines")
                .help("Number of bytes"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Suppress headers"),
        )
        .get_matches();

    let lines = matches
        .value_of("lines")
        .map(parse_num)
        .transpose()
        .map_err(|e| format!("illegal line count -- {}", e))?;
    let bytes = matches
        .value_of("bytes")
        .map(parse_num)
        .transpose()
        .map_err(|e| format!("illegal byte count -- {}", e))?;

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        lines: lines.unwrap(),
        bytes,
        quiet: matches.is_present("quiet"),
    })
}

fn count_lines_bytes(filename: &str) -> MyResult<(i64, i64)> {
    let num_bytes = metadata(filename)?.len() as i64;
    let num_lines = BufReader::new(File::open(filename)?).lines().count() as i64;
    Ok((num_lines, num_bytes))
}

fn get_start_index(take_val: TakeValue, total: i64) -> Option<u64> {
    let start = match take_val {
        PlusZero => Some(0),
        TakeNum(val) if val < 0 => Some(max(total + val, 0) as u64),
        TakeNum(val) => Some((val - 1) as u64),
    };
    start.filter(|v| total > 0 && *v < total as u64)
}

fn print_lines(mut file: impl BufRead, num_lines: TakeValue, total_lines: i64) -> MyResult<()> {
    if let Some(mut offset) = get_start_index(num_lines, total_lines) {
        let mut buf = String::new();
        while offset > 0 {
            file.read_line(&mut buf)?;
            offset -= 1;
        }

        buf.clear();
        loop {
            file.read_line(&mut buf)?;
            if buf.is_empty() {
                break;
            }

            print!("{}", buf);
            buf.clear();
        }
    }
    Ok(())
}

fn print_bytes<T>(mut file: T, num_bytes: TakeValue, total_bytes: i64) -> MyResult<()>
where
    T: Read + Seek,
{
    if let Some(offset) = get_start_index(num_bytes, total_bytes) {
        file.seek(SeekFrom::Start(offset))?;
        // let mut buf = [0u8; 1024];
        // loop {
        //     let n = file.read(&mut buf)?;
        //     if n == 0 {
        //         break;
        //     }
        //     std::io::stdout().write_all(&buf[..n])?;
        // }
        let mut buf = vec![];
        file.read_to_end(&mut buf)?;
        print!("{}", String::from_utf8_lossy(&buf));
    }
    Ok(())
}

pub fn run(config: Config) -> MyResult<()> {
    let print_header = |filename: &str, first: bool| {
        if !first {
            println!();
        }
        println!("==> {} <==", filename);
    };

    let mut printed_header = false;
    for filename in &config.files {
        match File::open(filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(file) => {
                let (total_lines, total_bytes) = count_lines_bytes(filename)?;
                if config.files.len() > 1 && !config.quiet {
                    print_header(filename, !printed_header);
                    printed_header = true;
                }
                let file = BufReader::new(file);
                if let Some(num_bytes) = config.bytes {
                    print_bytes(file, num_bytes, total_bytes)?;
                } else {
                    print_lines(file, config.lines, total_lines)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{count_lines_bytes, get_start_index, parse_num, PlusZero, TakeNum};

    #[test]
    fn test_parse_num() {
        // All integers should be interpreted as negative numbers
        let res = parse_num("3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        // A leading "+" should result in a positive number
        let res = parse_num("+3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(3));

        // An explicit "-" value should result in a negative number
        let res = parse_num("-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        // Zero is zero
        let res = parse_num("0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(0));

        // Plus zero is special
        let res = parse_num("+0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PlusZero);

        // Test boundaries
        let res = parse_num(&i64::MAX.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));
        let res = parse_num(&(i64::MIN + 1).to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));
        let res = parse_num(&format!("+{}", i64::MAX));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MAX));
        let res = parse_num(&i64::MIN.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN));

        // A floating-point value is invalid
        let res = parse_num("3.14");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "3.14");

        // Any noninteger string is invalid
        let res = parse_num("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "foo");
    }

    #[test]
    fn test_count_lines_bytes() {
        let res = count_lines_bytes("tests/inputs/one.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (1, 24));

        let res = count_lines_bytes("tests/inputs/ten.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (10, 49));
    }

    #[test]
    fn test_get_start_index() {
        // +0 from an empty file (0 lines/bytes) returns None
        assert_eq!(get_start_index(PlusZero, 0), None);

        // +0 from a nonempty file returns an index that
        // is one less than the number of lines/bytes
        assert_eq!(get_start_index(PlusZero, 1), Some(0));

        // Taking 0 lines/bytes returns None
        assert_eq!(get_start_index(TakeNum(0), 1), None);

        // Taking any lines/bytes from an empty file returns None
        assert_eq!(get_start_index(TakeNum(1), 0), None);

        // Taking more lines/bytes than is available returns None
        assert_eq!(get_start_index(TakeNum(2), 1), None);

        // When starting line/byte is less than total lines/bytes,
        // return one less than starting number
        assert_eq!(get_start_index(TakeNum(1), 10), Some(0));
        assert_eq!(get_start_index(TakeNum(2), 10), Some(1));
        assert_eq!(get_start_index(TakeNum(3), 10), Some(2));

        // When starting line/byte is negative and less than total,
        // return total - start
        assert_eq!(get_start_index(TakeNum(-1), 10), Some(9));
        assert_eq!(get_start_index(TakeNum(-2), 10), Some(8));
        assert_eq!(get_start_index(TakeNum(-3), 10), Some(7));

        // When starting line/byte is negative and more than total,
        // return 0 to print the whole file
        assert_eq!(get_start_index(TakeNum(-20), 10), Some(0));
    }
}
