use std::str::FromStr;

use ansi_term::Style;
use chrono::{Datelike, Days, Local, Months, NaiveDate, Weekday};
use clap::{App, Arg};
use itertools::{izip, Itertools};

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

#[derive(Debug)]
pub struct Config {
    month: Option<u32>,
    year: i32,
    today: NaiveDate,
}

fn parse_int<T: FromStr>(val: &str) -> MyResult<T> {
    T::from_str(val).map_err(|_| format!("Invalid integer \"{}\"", val).into())
}

fn parse_year(y: &str) -> MyResult<i32> {
    // match y.parse::<i32>() {
    match parse_int(y) {
        Ok(year) if (1..=9999).contains(&year) => Ok(year),
        Ok(year) => Err(format!("year \"{}\" not in the range 1 through 9999", year).into()),
        _ => Err(format!("Invalid year \"{}\"", y).into()),
    }
}

fn parse_month(val: &str) -> MyResult<u32> {
    // match val.parse::<u32>() {
    match parse_int(val) {
        Ok(month) if (1..=12).contains(&month) => Ok(month),
        Ok(month) => Err(format!("month \"{}\" not in the range 1 through 12", month).into()),
        _ => {
            let possible_months: Vec<_> = MONTHS
                .iter()
                .enumerate()
                .filter(|(_, m)| m.to_lowercase().starts_with(&val.to_lowercase()))
                .collect();

            match possible_months.as_slice() {
                &[(idx, _)] => Ok(idx as u32 + 1),
                _ => Err(format!("Invalid month \"{}\"", val).into()),
            }
        }
    }
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("calr")
        .version("0.1.0")
        .author("Tyson Liddell <tysonliddell@hotmail.com>")
        .about("Rust cal")
        .arg(
            Arg::with_name("month")
                .value_name("MONTH")
                .short("m")
                .short("month")
                .help("Month name or number (1-12)"),
        )
        .arg(
            Arg::with_name("show_current_year")
                .short("y")
                .long("year")
                .help("Show whole current year")
                .conflicts_with_all(&["month", "year"]),
        )
        .arg(
            Arg::with_name("year")
                .value_name("YEAR")
                .help("Year (1-9999)"),
        )
        .get_matches();

    let month = matches.value_of("month").map(parse_month).transpose()?;
    let year = matches.value_of("year").map(parse_year).transpose()?;
    let show_month = !matches.is_present("show_current_year") && year.is_none();

    let today = Local::now();
    Ok(Config {
        // clippy suggested to replace the following *_or functions with *_or_else
        // because *_or_else is lazily evaluated. i.e. it won't do the computation
        // if it doesn't have to!
        // month: month.or(show_month.then_some(today.month())),
        // year: year.unwrap_or(today.year()),
        month: month.or_else(|| show_month.then_some(today.month())),
        year: year.unwrap_or_else(|| today.year()),
        today: today.date_naive(),
    })
}

fn last_day_in_month(year: i32, month: u32) -> NaiveDate {
    let date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    date + Months::new(1) - Days::new(1)
}

fn format_month(year: i32, month: u32, print_year: bool, today: NaiveDate) -> Vec<String> {
    let mut lines = vec![];

    let mut title = MONTHS[(month - 1) as usize].to_string();
    if print_year {
        title.push_str(&format!(" {}", year));
    }
    let title = format!("{:^20} ", title);
    lines.push(title);
    let dow_headers = String::from("Su Mo Tu We Th Fr Sa ");
    lines.push(dow_headers);

    let mut date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let last_day_of_month = last_day_in_month(year, month);
    let today_style = Style::new().reverse();

    let num_leading_spaces_needed = date.weekday().num_days_from_sunday() * 3;
    let mut line: String = " ".repeat(num_leading_spaces_needed as usize);
    while date <= last_day_of_month {
        let num_str = format!("{:>2}", date.day());
        if date == today {
            line.push_str(&format!("{} ", today_style.paint(num_str)));
        } else {
            line.push_str(&format!("{} ", num_str));
        }

        if date.weekday() == Weekday::Sat {
            lines.push(line);
            line = String::new();
        }

        date = date.succ_opt().unwrap();
    }

    // add last line
    lines.push(format!("{:<21}", line));

    // add any needed extra blank lines
    while lines.len() < 8 {
        lines.push(" ".repeat(3 * 7));
    }

    // add a single space to to end of each line
    for line in &mut lines {
        line.push(' ');
    }

    lines
}

fn format_year(year: i32, today: NaiveDate) -> Vec<String> {
    let mut lines = vec![format!("{:>32}", year)];
    for (rows_done, months) in (1..=12).into_iter().chunks(3).into_iter().enumerate() {
        let (m1, m2, m3) = months
            .map(|month| format_month(year, month, false, today))
            .collect_tuple()
            .unwrap();
        for (m1, m2, m3) in izip!(m1.into_iter(), m2.into_iter(), m3.into_iter()) {
            lines.push([m1, m2, m3].join(""));
        }
        if rows_done < 3 {
            lines.push(String::from(""));
        }
    }
    lines
}

pub fn run(config: Config) -> MyResult<()> {
    if let Some(month) = config.month {
        for line in format_month(config.year, month, true, config.today) {
            println!("{}", line);
        }
    } else {
        for line in format_year(config.year, config.today) {
            println!("{}", line);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{format_month, last_day_in_month, parse_int, parse_month, parse_year, NaiveDate};

    #[test]
    fn test_parse_int() {
        // Parse positive int as usize
        let res = parse_int::<usize>("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1usize);

        // Parse negative int as i32
        let res = parse_int::<i32>("-1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), -1i32);

        // Fail on a string
        let res = parse_int::<i64>("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Invalid integer \"foo\"");
    }

    #[test]
    fn test_parse_year() {
        let res = parse_year("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1i32);

        let res = parse_year("9999");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 9999i32);

        let res = parse_year("0");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "year \"0\" not in the range 1 through 9999"
        );

        let res = parse_year("10000");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "year \"10000\" not in the range 1 through 9999"
        );

        let res = parse_year("foo");
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_month() {
        let res = parse_month("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1u32);

        let res = parse_month("12");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 12u32);

        let res = parse_month("jan");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1u32);

        let res = parse_month("0");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "month \"0\" not in the range 1 through 12"
        );

        let res = parse_month("13");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "month \"13\" not in the range 1 through 12"
        );

        let res = parse_month("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Invalid month \"foo\"");
    }

    #[test]
    fn test_format_month() {
        let today = NaiveDate::from_ymd(0, 1, 1);
        let leap_february = vec![
            "   February 2020      ",
            "Su Mo Tu We Th Fr Sa  ",
            "                   1  ",
            " 2  3  4  5  6  7  8  ",
            " 9 10 11 12 13 14 15  ",
            "16 17 18 19 20 21 22  ",
            "23 24 25 26 27 28 29  ",
            "                      ",
        ];
        assert_eq!(format_month(2020, 2, true, today), leap_february);
        let may = vec![
            "        May           ",
            "Su Mo Tu We Th Fr Sa  ",
            "                1  2  ",
            " 3  4  5  6  7  8  9  ",
            "10 11 12 13 14 15 16  ",
            "17 18 19 20 21 22 23  ",
            "24 25 26 27 28 29 30  ",
            "31                    ",
        ];
        assert_eq!(format_month(2020, 5, false, today), may);
        let april_hl = vec![
            "     April 2021       ",
            "Su Mo Tu We Th Fr Sa  ",
            "             1  2  3  ",
            " 4  5  6 \u{1b}[7m 7\u{1b}[0m  8  9 10  ",
            "11 12 13 14 15 16 17  ",
            "18 19 20 21 22 23 24  ",
            "25 26 27 28 29 30     ",
            "                      ",
        ];
        let today = NaiveDate::from_ymd(2021, 4, 7);
        assert_eq!(format_month(2021, 4, true, today), april_hl);
        let april_hl = vec![
            "     April 2021       ",
            "Su Mo Tu We Th Fr Sa  ",
            "             1 \u{1b}[7m 2\u{1b}[0m  3  ",
            " 4  5  6  7  8  9 10  ",
            "11 12 13 14 15 16 17  ",
            "18 19 20 21 22 23 24  ",
            "25 26 27 28 29 30     ",
            "                      ",
        ];
        let today = NaiveDate::from_ymd(2021, 4, 2);
        assert_eq!(format_month(2021, 4, true, today), april_hl);
    }

    #[test]
    fn test_last_day_in_month() {
        assert_eq!(last_day_in_month(2020, 1), NaiveDate::from_ymd(2020, 1, 31));
        assert_eq!(last_day_in_month(2020, 2), NaiveDate::from_ymd(2020, 2, 29));
        assert_eq!(last_day_in_month(2020, 4), NaiveDate::from_ymd(2020, 4, 30));
    }
}
