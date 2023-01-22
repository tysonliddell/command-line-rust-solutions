use clap::{App, Arg, ArgGroup};

use crate::Extract::*;
use std::ops::Range;

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

fn parse_positive_int(val: &str) -> MyResult<usize> {
    if val.contains('+') {
        return Err(From::from("cannot parse string with +"));
    }

    match val.parse() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(From::from(val)),
    }
}

fn parse_pos(ranges: &str) -> MyResult<PositionList> {
    ranges
        .split(',')
        .map(|range| {
            let mut range = range
                .splitn(2, '-')
                .map(parse_positive_int)
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
                .value_name("LIST")
                .short("b")
                .long("bytes")
                .help("Selected bytes")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("chars")
                .value_name("LIST")
                .short("c")
                .long("characters")
                .takes_value(true)
                .help("Selected characters"),
        )
        .arg(
            Arg::with_name("fields")
                .value_name("LIST")
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

    let mut iter = matches.value_of("delim").unwrap().bytes();
    let delimiter = iter.next().unwrap();
    if iter.next().is_some() {
        return Err(From::from("Delimiter is larger than 1 byte"));
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
        delimiter,
        extract,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    println!("{:?}", config);
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::parse_pos;

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
}
