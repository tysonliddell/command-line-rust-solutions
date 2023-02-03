use std::{
    fs::{metadata, File},
    io::{BufRead, BufReader},
    path::PathBuf,
};

use clap::{App, Arg};
use rand::{seq::SliceRandom, SeedableRng};
use regex::{Regex, RegexBuilder};
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct Config {
    sources: Vec<String>,
    pattern: Option<Regex>,
    seed: Option<u64>,
}

#[derive(Debug)]
struct Fortune {
    source: String,
    text: String,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("fortuner")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust fortune")
        .arg(
            Arg::with_name("sources")
                .value_name("FILE")
                .required(true)
                .multiple(true)
                .help("Input files or directories"),
        )
        .arg(
            Arg::with_name("pattern")
                .value_name("PATTERN")
                .short("m")
                .long("pattern")
                .help("Pattern"),
        )
        .arg(
            Arg::with_name("insensitive")
                .short("i")
                .long("insensitive")
                .help("Case-insensitive pattern matching"),
        )
        .arg(
            Arg::with_name("seed")
                .value_name("SEED")
                .short("s")
                .long("seed")
                .help("Random seed"),
        )
        .get_matches();

    let seed: Option<u64> = matches
        .value_of("seed")
        .map(|u| {
            u.parse()
                .map_err(|_| format!("\"{}\" not a valid integer", u))
        })
        .transpose()?;

    let is_insensitive = matches.is_present("insensitive");
    let pattern = matches
        .value_of("pattern")
        .map(|p| {
            RegexBuilder::new(p)
                .case_insensitive(is_insensitive)
                .build()
                .map_err(|_| format!("Invalid --pattern \"{}\"", p))
        })
        .transpose()?;

    Ok(Config {
        sources: matches.values_of_lossy("sources").unwrap(),
        pattern,
        seed,
    })
}

fn find_files(paths: &[String]) -> MyResult<Vec<PathBuf>> {
    let mut result = vec![];
    for filename in paths {
        match metadata(filename) {
            Err(e) => return Err(From::from(format!("{}: {}", filename, e))),
            Ok(_) => {
                let child_paths = WalkDir::new(filename)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|v| v.file_type().is_file())
                    .map(|child| child.into_path());
                result.extend(child_paths);
            }
        }
    }

    result.sort();
    result.dedup();
    Ok(result)
}

fn read_next_fortune(file: &mut impl BufRead) -> MyResult<Option<String>> {
    let mut fortune = String::new();
    let mut buf = String::new();

    let trim_in_place = |mut s: String| {
        s.truncate(s.trim_end().len());
        s
    };
    loop {
        if file.read_line(&mut buf)? == 0 {
            return match fortune.as_str() {
                "" => Ok(None),
                _ => Ok(Some(trim_in_place(fortune))),
            };
        } else if buf.trim_end() == "%" {
            return Ok(Some(trim_in_place(fortune)));
        }
        fortune.push_str(&buf);
        buf.clear();
    }
}

fn read_fortunes(paths: &[PathBuf]) -> MyResult<Vec<Fortune>> {
    let mut fortunes = vec![];
    for path in paths {
        let mut file = BufReader::new(File::open(path)?);
        loop {
            match read_next_fortune(&mut file)? {
                None => break,
                Some(fortune) if !fortune.is_empty() => fortunes.push(Fortune {
                    source: path.file_name().unwrap().to_string_lossy().into_owned(),
                    text: fortune,
                }),
                _ => (),
            }
        }
    }
    Ok(fortunes)
}

fn pick_fortune(fortunes: &[Fortune], seed: Option<u64>) -> Option<String> {
    // I don't like this code. Looking forward to seeing a better way to
    // assign either PRNGs to the same variable.
    match seed {
        Some(seed) => {
            let mut prng = rand::rngs::StdRng::seed_from_u64(seed);
            fortunes.choose(&mut prng).map(|f| f.text.clone())
        }
        None => {
            let mut prng = rand::thread_rng();
            fortunes.choose(&mut prng).map(|f| f.text.clone())
        }
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let files = find_files(&config.sources)?;
    let fortunes = read_fortunes(&files)?;

    if let Some(pattern) = config.pattern {
        let fortunes = fortunes.iter().filter(|f| pattern.is_match(&f.text));
        let mut curr_source = "";
        for fortune in fortunes {
            if fortune.source != curr_source {
                curr_source = &fortune.source;
                eprintln!("({})", fortune.source);
                eprintln!("%");
            }
            println!("{}", fortune.text);
            println!("%");
        }
    } else {
        match pick_fortune(&fortunes, config.seed) {
            Some(fortune) => println!("{}", fortune),
            None => println!("No fortunes found"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{find_files, pick_fortune, read_fortunes, Fortune, PathBuf};

    #[test]
    fn test_find_files() {
        // Verify that the function finds a file known to exist
        let res = find_files(&["./tests/inputs/jokes".to_string()]);
        assert!(res.is_ok());

        let files = res.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files.get(0).unwrap().to_string_lossy(),
            "./tests/inputs/jokes"
        );

        // Fails to find a bad file
        let res = find_files(&["/path/does/not/exist".to_string()]);
        assert!(res.is_err());

        // Finds all the input files, excludes ".dat"
        let res = find_files(&["./tests/inputs".to_string()]);
        assert!(res.is_ok());

        // Check number and order of files
        let files = res.unwrap();
        assert_eq!(files.len(), 5);
        let first = files.get(0).unwrap().display().to_string();
        assert!(first.contains("ascii-art"));
        let last = files.last().unwrap().display().to_string();
        assert!(last.contains("quotes"));

        // Test for multiple sources, path must be unique and sorted
        let res = find_files(&[
            "./tests/inputs/jokes".to_string(),
            "./tests/inputs/ascii-art".to_string(),
            "./tests/inputs/jokes".to_string(),
        ]);
        assert!(res.is_ok());
        let files = res.unwrap();
        assert_eq!(files.len(), 2);
        if let Some(filename) = files.first().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "ascii-art".to_string())
        }
        if let Some(filename) = files.last().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "jokes".to_string())
        }
    }

    #[test]
    fn test_read_fortunes() {
        // One input file
        let res = read_fortunes(&[PathBuf::from("./tests/inputs/jokes")]);
        assert!(res.is_ok());

        if let Ok(fortunes) = res {
            // Correct number and sorting
            assert_eq!(fortunes.len(), 6);
            assert_eq!(
                fortunes.first().unwrap().text,
                "Q. What do you call a head of lettuce in a shirt and tie?\n\
                A. Collared greens."
            );
            assert_eq!(
                fortunes.last().unwrap().text,
                "Q: What do you call a deer wearing an eye patch?\n\
                A: A bad idea (bad-eye deer)."
            );
        }

        // Multiple input files
        let res = read_fortunes(&[
            PathBuf::from("./tests/inputs/jokes"),
            PathBuf::from("./tests/inputs/quotes"),
        ]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 11);
    }

    #[test]
    fn test_pick_fortune() {
        // Create a slice of fortunes
        let fortunes = &[
            Fortune {
                source: "fortunes".to_string(),
                text: "You cannot achieve the impossible without \
                              attempting the absurd."
                    .to_string(),
            },
            Fortune {
                source: "fortunes".to_string(),
                text: "Assumption is the mother of all screw-ups.".to_string(),
            },
            Fortune {
                source: "fortunes".to_string(),
                text: "Neckties strangle clear thinking.".to_string(),
            },
        ];
        // Pick a fortune with a seed
        assert_eq!(
            pick_fortune(fortunes, Some(1)).unwrap(),
            "Neckties strangle clear thinking.".to_string()
        );
    }
}
