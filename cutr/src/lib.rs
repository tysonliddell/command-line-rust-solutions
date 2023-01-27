use crate::Extract::*;
use clap::{App, Arg, ArgGroup};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use std::{
    cmp::min,
    fs::File,
    io::{self, BufRead, BufReader},
    num::NonZeroUsize,
    ops::Range,
};

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;
type PositionList = Vec<Range<usize>>;

#[derive(Debug)]
pub enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    delimiter: u8,
    extract: Extract,
}

fn parse_index(val: &str) -> MyResult<usize> {
    let value_error = || From::from(format!("illegal index value: \"{}\"", val));
    val.starts_with('+')
        .then(|| Err(value_error()))
        .unwrap_or_else(|| {
            val.parse::<NonZeroUsize>()
                .map(From::from)
                .map_err(|_| value_error())
        })
}

fn parse_pos(ranges: &str) -> MyResult<PositionList> {
    ranges
        .split(',')
        .map(|range| {
            let mut range = range
                .splitn(2, '-')
                .map(parse_index)
                .collect::<Result<Vec<usize>, _>>()
                .map_err(|_| format!("illegal list value: \"{}\"", range))?;

            if range.len() == 1 {
                range.push(range[0])
            } else if range[1] <= range[0] {
                return Err(From::from(format!(
                    "First number in range ({}) must be lower than second number ({})",
                    range[0], range[1]
                )));
            }

            Ok(range[0] - 1..range[1])
        })
        .collect()
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("cutr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust cut")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .multiple(true)
                .default_value("-")
                .help("Input file(s)"),
        )
        .arg(
            Arg::with_name("bytes")
                .value_name("BYTES")
                .short("b")
                .long("bytes")
                .help("Selected bytes")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("chars")
                .value_name("CHARS")
                .short("c")
                .long("characters")
                .takes_value(true)
                .help("Selected characters"),
        )
        .arg(
            Arg::with_name("fields")
                .value_name("FIELDS")
                .short("f")
                .long("fields")
                .takes_value(true)
                .help("Selected fields"),
        )
        .arg(
            Arg::with_name("delim")
                .value_name("DELIM")
                .short("d")
                .long("delimiter")
                .takes_value(true)
                .default_value("\t")
                .hide_default_value(true)
                .help("use DELIM instead of TAB for field delimiter"),
        )
        .group(
            ArgGroup::with_name("list")
                .args(&["bytes", "chars", "fields"])
                .required(true)
                .multiple(false),
        )
        .get_matches();

    let delimiter = matches.value_of("delim").unwrap();
    if delimiter.len() != 1 {
        return Err(From::from(format!(
            "--delim \"{}\" must be a single byte",
            delimiter
        )));
    }

    let pos_list = parse_pos(matches.value_of("list").unwrap())?;
    let extract = if matches.is_present("bytes") {
        Bytes(pos_list)
    } else if matches.is_present("chars") {
        Chars(pos_list)
    } else {
        Fields(pos_list)
    };

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        delimiter: *delimiter.as_bytes().first().unwrap(),
        extract,
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn extract_chars(line: &str, char_pos: &[Range<usize>]) -> String {
    let char_vec: Vec<_> = line.chars().collect();
    char_pos
        .iter()
        .filter(|r| r.start < char_vec.len())
        .flat_map(|r| &char_vec[r.start..min(r.end, char_vec.len())])
        .collect()
}

fn extract_bytes(line: &str, byte_pos: &[Range<usize>]) -> String {
    let bytes = line.as_bytes();
    let bytes: Vec<u8> = byte_pos
        .iter()
        .filter(|r| r.start < bytes.len())
        .flat_map(|r| &bytes[r.start..min(r.end, bytes.len())])
        .copied()
        .collect();
    String::from_utf8_lossy(&bytes).to_string()
}

fn extract_fields(record: &StringRecord, field_pos: &[Range<usize>]) -> Vec<String> {
    let valid_record_range = |r: &Range<usize>| r.start..min(r.end, record.len());
    field_pos
        .iter()
        .filter(|r| r.start < record.len())
        .flat_map(valid_record_range)
        .map(|i| String::from(&record[i]))
        .collect()
}

pub fn run(config: Config) -> MyResult<()> {
    for filename in config.files {
        match open(&filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(file) => match &config.extract {
                Bytes(pos_list) => {
                    for line in file.lines() {
                        println!("{}", extract_bytes(&line?, pos_list));
                    }
                }
                Chars(pos_list) => {
                    for line in file.lines() {
                        println!("{}", extract_chars(&line?, pos_list));
                    }
                }
                Fields(pos_list) => {
                    let mut reader = ReaderBuilder::new()
                        .delimiter(config.delimiter)
                        .has_headers(false)
                        .from_reader(file);
                    let mut writer = WriterBuilder::new()
                        .delimiter(config.delimiter)
                        .has_headers(false)
                        .from_writer(io::stdout());
                    for record in reader.records() {
                        let fields = extract_fields(&record?, pos_list);
                        writer.write_record(&fields)?;
                        writer.flush()?;
                    }
                }
            },
        }
    }
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::{extract_bytes, extract_chars, extract_fields, parse_pos};
    use csv::StringRecord;

    #[test]
    fn test_parse_pos() {
        // The empty string is an error
        assert!(parse_pos("").is_err());

        // Zero is an error
        let res = parse_pos("0");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"",);

        let res = parse_pos("0-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0-1\"",);

        // A leading + is an error
        let res = parse_pos("+1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1\"",);

        let res = parse_pos("+1-2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1-2\"",);

        let res = parse_pos("1-+2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-+2\"",);

        // Any non-number is an error
        let res = parse_pos("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1-a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-a\"",);

        let res = parse_pos("a-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a-1\"",);

        // Wonky ranges
        assert!(parse_pos("-").is_err());
        assert!(parse_pos(",").is_err());
        assert!(parse_pos("1,").is_err());
        assert!(parse_pos("1-").is_err());
        assert!(parse_pos("1-1-1").is_err());
        assert!(parse_pos("1-1-a").is_err());

        // First number must be less than second
        let res = parse_pos("1-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (1) must be lower than second number (1)",
        );

        let res = parse_pos("2-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (2) must be lower than second number (1)",
        );

        // All the following are acceptable
        let res = parse_pos("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("01");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("1,3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("001,0003");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("1-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("0001-03");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("1,7,3-5");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_pos("15,19-20");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }

    #[test]
    fn test_extract_chars() {
        assert_eq!(extract_chars("", &[0..1]), "".to_string());
        assert_eq!(extract_chars("ábc", &[0..1]), "á".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 2..3]), "ác".to_string());
        assert_eq!(extract_chars("ábc", &[0..3]), "ábc".to_string());
        assert_eq!(extract_chars("ábc", &[2..3, 1..2]), "cb".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 1..2, 4..5]), "áb".to_string());
    }

    #[test]
    fn test_extract_bytes() {
        assert_eq!(extract_bytes("ábc", &[0..1]), "�".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2]), "á".to_string());
        assert_eq!(extract_bytes("ábc", &[0..3]), "áb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..4]), "ábc".to_string());
        assert_eq!(extract_bytes("ábc", &[3..4, 2..3]), "cb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2, 5..6]), "á".to_string());
    }

    #[test]
    fn test_extract_fields() {
        let rec = StringRecord::from(vec!["Captain", "Sham", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2]), &["Sham"]);
        assert_eq!(extract_fields(&rec, &[0..1, 2..3]), &["Captain", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1, 3..4]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2, 0..1]), &["Sham", "Captain"]);
    }
}
