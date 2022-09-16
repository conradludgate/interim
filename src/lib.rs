//! ## Parsing English Dates
//!
//! I've always admired the ability of the GNU `date` command to
//! convert "English" expressions to dates and times with `date -d expr`.
//! `chrono-english` does similar expressions, although with extensions, so
//! that for instance you can specify both the day and the time "next friday 8pm".
//! No attempt at full natural language parsing is made - only a limited set of
//! patterns is supported.
//!
//! ## Supported Formats
//!
//! `chrono-english` does _absolute_ dates:  ISO-like dates "2018-04-01" and the month name forms
//! "1 April 2018" and "April 1, 2018". (There's no ambiguity so both of these forms are fine)
//!
//! The informal "01/04/18" or American form "04/01/18" is supported.
//! There is a `Dialect` enum to specify what kind of date English you would like to speak.
//! Both short and long years are accepted in this form; short dates pivot between 1940 and 2040.
//!
//! Then there are are _relative_ dates like 'April 1' and '9/11' (this
//! if using `Dialect::Us`). The current year is assumed, but this can be modified by 'next'
//! and 'last'. For instance, it is now the 13th of March, 2018: 'April 1' and 'next April 1'
//! are in 2018; 'last April 1' is in 2017.
//!
//! Another relative form is simply a month name
//! like 'apr' or 'April' (case-insensitive, only first three letters significant) where the
//! day is assumed to be the 1st.
//!
//! A week-day works in the same way: 'friday' means this
//! coming Friday, relative to today. 'last Friday' is unambiguous,
//! but 'next Friday' has different meanings; in the US it means the same as 'Friday'
//! but otherwise it means the Friday of next week (plus 7 days)
//!
//! Date and time can be specified also by a number of time units. So "2 days", "3 hours".
//! Again, first three letters, but 'd','m' and 'y' are understood (so "3h"). We make
//! a distinction between _second_ intervals (seconds,minutes,hours,days,weeks) and _month_
//! intervals (months,years).  Month intervals always give us the same date, if possible
//! But adding a month to "30 Jan" will give "28 Feb" or "29 Feb" depending if a leap year.
//!
//! Finally, dates may be followed by time. Either 'formal' like 18:03, with optional
//! second (like 18:03:40) or 'informal' like 6.03pm. So one gets "next friday 8pm' and so
//! forth.
//!
//! ## API
//!
//! There are two entry points: `parse_date_string` and `parse_duration`. The
//! first is given the date string, a `DateTime` from which relative dates and
//! times operate, and a dialect (either `Dialect::Uk` or `Dialect::Us`
//! currently.) The base time also specifies the desired timezone.
//!
//! ```ignore
//! use chrono_english2::{parse_date_string, Dialect};
//! use chrono::Local;
//!
//! let date_time = parse_date_string("next friday 8pm", Local::now(), Dialect::Uk)?;
//! println!("{}", date_time.format("%c"));
//! ```
//!
//! There is a little command-line program `parse-date` in the `examples` folder which can be used to play
//! with these expressions.
//!
//! The other function, `parse_duration`, lets you access just the relative part
//! of a string like 'two days ago' or '12 hours'. If successful, returns an
//! `Interval`, which is a number of seconds, days, or months.
//!
//! ```
//! use chrono_english2::{parse_duration, Interval};
//!
//! assert_eq!(parse_duration("15m ago").unwrap(), Interval::Seconds(-15 * 60));
//! ```
//!

use chrono::prelude::*;

mod errors;
mod parser;
mod types;
use types::*;

pub use errors::{DateError, DateResult};
pub use types::Interval;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    Uk,
    Us,
}

pub fn parse_date_string<Tz: TimeZone>(
    s: &str,
    now: DateTime<Tz>,
    dialect: Dialect,
) -> DateResult<DateTime<Tz>> {
    let d = parser::DateParser::new(s).dialect(dialect).parse()?;

    // we may have explicit hour:minute:sec
    let tspec = match d.time {
        Some(tspec) => tspec,
        None => TimeSpec::new_empty(),
    };
    let date_time = if let Some(dspec) = d.date {
        dspec
            .into_date_time(now, tspec, dialect)
            .ok_or("bad date")?
    } else {
        // no date, time set for today's date
        tspec.into_date_time(now.date()).ok_or("bad time")?
    };
    Ok(date_time)
}

pub fn parse_duration(s: &str) -> DateResult<Interval> {
    let d = parser::DateParser::new(s).parse()?;

    if d.time.is_some() {
        return Err(DateError::new("unexpected time component"));
    }

    // shouldn't happen, but.
    if d.date.is_none() {
        return Err(DateError::new("could not parse date"));
    }

    match d.date.unwrap() {
        DateSpec::Absolute(_) => Err(DateError::new("unexpected absolute date")),
        DateSpec::FromName(_) => Err(DateError::new("unexpected date component")),
        DateSpec::Relative(skip) => Ok(skip.into_interval()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let base = Utc.ymd(2018, 3, 21).and_hms(11, 00, 00);

        macro_rules! assert_date_string {
            ($s:literal, $dialect:ident, $expect:literal) => {
                let input = $s;
                let expected: &str = $expect;
                match parse_date_string(input, base, Dialect::$dialect) {
                    Err(e) => {
                        panic!("unexpected error attempting to parse {input:?}\n\t{e:?}")
                    }
                    Ok(date) => {
                        let date = date.format("%+").to_string();
                        if date != expected {
                            panic!("unexpected output attempting to parse {input:?}.\nexpected: {expected:?}\n  parsed: {date:?}");
                        }
                    }
                }
            };
        }

        // Day of week - relative to today. May have a time part
        assert_date_string!("friday", Uk, "2018-03-23T00:00:00+00:00");
        assert_date_string!("friday 10:30", Uk, "2018-03-23T10:30:00+00:00");
        assert_date_string!("friday 8pm", Uk, "2018-03-23T20:00:00+00:00");

        // The day of week is the _next_ day after today, so "Tuesday" is the next Tuesday after Wednesday
        assert_date_string!("tues", Uk, "2018-03-27T00:00:00+00:00");

        // The expression 'next Monday' is ambiguous; in the US it means the day following (same as 'Monday')
        // (This is how the `date` command interprets it)
        assert_date_string!("next mon", Us, "2018-03-26T00:00:00+00:00");
        // but otherwise it means the day in the next week..
        assert_date_string!("next mon", Uk, "2018-04-02T00:00:00+00:00");

        assert_date_string!("last fri 9.30", Uk, "2018-03-16T09:30:00+00:00");

        // date expressed as month, day - relative to today. May have a time part
        assert_date_string!("9/11", Us, "2018-09-11T00:00:00+00:00");
        assert_date_string!("last 9/11", Us, "2017-09-11T00:00:00+00:00");
        assert_date_string!("last 9/11 9am", Us, "2017-09-11T09:00:00+00:00");
        assert_date_string!("April 1 8.30pm", Uk, "2018-04-01T20:30:00+00:00");

        // advance by time unit from today
        // without explicit time, use base time - otherwise override
        assert_date_string!("2d", Uk, "2018-03-23T11:00:00+00:00");
        assert_date_string!("2d 03:00", Uk, "2018-03-23T03:00:00+00:00");
        assert_date_string!("3 weeks", Uk, "2018-04-11T11:00:00+00:00");
        assert_date_string!("3h", Uk, "2018-03-21T14:00:00+00:00");
        assert_date_string!("6 months", Uk, "2018-09-21T00:00:00+00:00");
        assert_date_string!("6 months ago", Uk, "2017-09-21T00:00:00+00:00");
        assert_date_string!("3 hours ago", Uk, "2018-03-21T08:00:00+00:00");
        assert_date_string!(" -3h", Uk, "2018-03-21T08:00:00+00:00");
        assert_date_string!(" -3 month", Uk, "2017-12-21T00:00:00+00:00");

        // absolute date with year, month, day - formal ISO and informal UK or US
        assert_date_string!("2017-06-30", Uk, "2017-06-30T00:00:00+00:00");
        assert_date_string!("30/06/17", Uk, "2017-06-30T00:00:00+00:00");
        assert_date_string!("06/30/17", Us, "2017-06-30T00:00:00+00:00");

        // may be followed by time part, formal and informal
        assert_date_string!("2017-06-30 08:20:30", Uk, "2017-06-30T08:20:30+00:00");
        assert_date_string!(
            "2017-06-30 08:20:30 +02:00",
            Uk,
            "2017-06-30T06:20:30+00:00"
        );
        assert_date_string!("2017-06-30 08:20:30 +0200", Uk, "2017-06-30T06:20:30+00:00");
        assert_date_string!("2017-06-30T08:20:30Z", Uk, "2017-06-30T08:20:30+00:00");
        assert_date_string!("2017-06-30T08:20:30", Uk, "2017-06-30T08:20:30+00:00");
        assert_date_string!("2017-06-30 8.20", Uk, "2017-06-30T08:20:00+00:00");
        assert_date_string!("2017-06-30 8.30pm", Uk, "2017-06-30T20:30:00+00:00");
        assert_date_string!("2017-06-30 8:30pm", Uk, "2017-06-30T20:30:00+00:00");
        assert_date_string!("2017-06-30 2am", Uk, "2017-06-30T02:00:00+00:00");
        assert_date_string!("30 June 2018", Uk, "2018-06-30T00:00:00+00:00");
        assert_date_string!("June 30, 2018", Uk, "2018-06-30T00:00:00+00:00");
        assert_date_string!("June   30,    2018", Uk, "2018-06-30T00:00:00+00:00");
    }

    #[test]
    fn durations() {
        macro_rules! assert_duration {
            ($s:literal, $expect:expr) => {
                let dur = parse_duration($s).unwrap();
                assert_eq!(dur, $expect);
            };
        }
        macro_rules! assert_duration_err {
            ($s:literal, $expect:literal) => {
                let err = parse_duration($s).unwrap_err().to_string();
                assert_eq!(err, $expect);
            };
        }

        assert_duration!("6h", Interval::Seconds(6 * 3600));
        assert_duration!("4 hours ago", Interval::Seconds(-4 * 3600));
        assert_duration!("5 min", Interval::Seconds(5 * 60));
        assert_duration!("10m", Interval::Seconds(10 * 60));
        assert_duration!("15m ago", Interval::Seconds(-15 * 60));

        assert_duration!("1 day", Interval::Days(1));
        assert_duration!("2 days ago", Interval::Days(-2));
        assert_duration!("3 weeks", Interval::Days(21));
        assert_duration!("2 weeks ago", Interval::Days(-14));

        assert_duration!("1 month", Interval::Months(1));
        assert_duration!("6 months", Interval::Months(6));
        assert_duration!("8 years", Interval::Months(12 * 8));

        // errors
        assert_duration_err!("2020-01-01", "unexpected absolute date");
        assert_duration_err!("2 days 15:00", "unexpected time component");
        assert_duration_err!("tuesday", "unexpected date component");
        assert_duration_err!("bananas", "expected week day or month name");
    }
}
