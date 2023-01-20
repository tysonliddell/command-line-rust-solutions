use std::io::{self, BufRead, BufReader, BufWriter, Write};

use std::fs::File;

use clap::{App, Arg};

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct Config {
    in_file: String,
    out_file: Option<String>,
    count: bool,
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn open_out(filename: &Option<String>) -> MyResult<Box<dyn Write>> {
    match filename {
        Some(f) => Ok(Box::new(BufWriter::new(File::create(f)?))),
        None => Ok(Box::new(BufWriter::new(io::stdout()))),
    }
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("uniqr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust uniq")
        .arg(
            Arg::with_name("in_file")
                .value_name("IN_FILE")
                .default_value("-")
                .help("Input file"),
        )
        .arg(
            Arg::with_name("out_file")
                .value_name("OUT_FILE")
                .help("Output file"),
        )
        .arg(
            Arg::with_name("count")
                .short("c")
                .long("count")
                .help("Show counts")
                .takes_value(false),
        )
        .get_matches();

    Ok(Config {
        in_file: matches.value_of("in_file").unwrap().to_string(),
        out_file: matches.value_of("out_file").map(String::from),
        count: matches.is_present("count"),
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let mut file = open(&config.in_file).map_err(|e| format!("{}: {}", config.in_file, e))?;
    let mut outfile = open_out(&config.out_file)
        .map_err(|e| format!("{}: {}", config.out_file.unwrap_or("stdout".to_string()), e))?;

    // let mut prev_line = None;
    let mut prev_line = String::new();
    let mut curr_line = String::new();

    file.read_line(&mut prev_line)?;
    let mut count: usize = 0;

    if prev_line.len() == 0 {
        // empty input
        return Ok(());
    }

    loop {
        count += 1;
        let bytes = file.read_line(&mut curr_line)?;
        if !curr_line.ends_with('\n') && prev_line[..prev_line.len() - 1] == curr_line {
            curr_line.push('\n');
        }
        if curr_line != prev_line {
            let prefix = if config.count {
                format!("{:>4} ", count)
            } else {
                "".to_string()
            };

            outfile.write(format!("{}{}", prefix, prev_line).as_bytes())?;
            prev_line = curr_line.clone();
            count = 0;
        }

        if bytes == 0 {
            break;
        }

        // print!("{}", buf);
        // prev_line = curr_line.clone();
        curr_line.clear();
    }

    outfile.flush()?;
    Ok(())
}
