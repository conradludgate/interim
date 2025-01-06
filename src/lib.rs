//! # interim
//!
//! interim started as a fork, but ended up being a complete over-haul of [chrono-english](https://github.com/stevedonovan/chrono-english).
//!
//! The API surface is the same, and all the original tests from chrono-english still pass, although there's some key differences
//!
//! ## Improvements
//!
//! Why use interim over chrono-english?
//!
//! 1. chrono-english is not actively maintained: <https://github.com/stevedonovan/chrono-english/issues/22>
//! 2. interim simplifies a lot of the code, removing a lot of potential panics and adds some optimisations.
//! 3. supports `no_std`, as well as the `time` and `jiff` crates
//!
//! ## Features
//!
//! * `std`: This crate is `no_std` compatible. Disable the default-features to disable the std-lib features (just error reporting)
//! * `time`: This crate is compatible with the [time crate](https://github.com/time-rs/time).
//! * `chrono`: This crate is compatible with the [chrono crate](https://github.com/chronotope/chrono).
//! * `jiff_0_1`: This crate is compatible with the [jiff crate](https://github.com/BurntSushi/jiff).
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
//! use interim::{parse_date_string, Dialect};
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
//! use interim::{parse_duration, Interval};
//!
//! assert_eq!(parse_duration("15m ago").unwrap(), Interval::Seconds(-15 * 60));
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]
#![warn(clippy::pedantic)]
#![allow(
    clippy::if_not_else,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

#[cfg(test)]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

/// A collection of traits to abstract over date-time implementations
pub mod datetime;
mod errors;
mod parser;
mod types;

use datetime::DateTime;
pub use errors::{DateError, DateResult};
pub use types::Interval;
use types::{DateSpec, DateTimeSpec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Form of english dates to parse
pub enum Dialect {
    Uk,
    Us,
}

/// Parse a date-time from the text, potentially relative to `now`. Accepts
/// a [`Dialect`] to support some slightly different text parsing behaviour.
///
/// ```
/// use interim::{parse_date_string, Dialect};
/// use chrono::{Utc, TimeZone};
///
/// let now = Utc.with_ymd_and_hms(2022, 9, 17, 13, 27, 0).unwrap();
/// let this_friday = parse_date_string("friday 8pm", now, Dialect::Uk).unwrap();
///
/// assert_eq!(this_friday, Utc.with_ymd_and_hms(2022, 9, 23, 20, 0, 0).unwrap());
/// ```
pub fn parse_date_string<Dt: DateTime>(s: &str, now: Dt, dialect: Dialect) -> DateResult<Dt> {
    into_date_string(parser::DateParser::new(s).parse(dialect)?, now, dialect)
}

fn into_date_string<Dt: DateTime>(d: DateTimeSpec, now: Dt, dialect: Dialect) -> DateResult<Dt> {
    // we may have explicit hour:minute:sec
    if let Some(dspec) = d.date {
        dspec
            .into_date_time(now, d.time, dialect)
            .ok_or(DateError::MissingDate)
    } else if let Some(tspec) = d.time {
        let (tz, date, _) = now.split();
        // no date, use todays date
        tspec.into_date_time(tz, date).ok_or(DateError::MissingTime)
    } else {
        Err(DateError::MissingTime)
    }
}

/// Parse an [`Interval`] from the text
///
/// ```
/// use interim::{parse_duration, Interval};
/// use chrono::{Utc, TimeZone};
///
/// let now = Utc.with_ymd_and_hms(2022, 9, 17, 13, 27, 0).unwrap();
/// let week_ago = parse_duration("1 week ago").unwrap();
/// let minutes = parse_duration("10m").unwrap();
///
/// assert_eq!(week_ago, Interval::Days(-7));
/// assert_eq!(minutes, Interval::Seconds(10*60));
/// ```
pub fn parse_duration(s: &str) -> DateResult<Interval> {
    let d = parser::DateParser::new(s).parse(Dialect::Uk)?;

    if d.time.is_some() {
        return Err(DateError::UnexpectedTime);
    }

    match d.date {
        Some(DateSpec::Relative(skip)) => Ok(skip),
        Some(DateSpec::Absolute(_)) => Err(DateError::UnexpectedAbsoluteDate),
        Some(DateSpec::FromName(..)) => Err(DateError::UnexpectedDate),
        None => Err(DateError::MissingDate),
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]

    use crate::{parse_duration, DateError, Dialect, Interval};
    use alloc::string::String;
    use alloc::string::ToString;

    #[cfg(feature = "chrono_0_4")]
    #[track_caller]
    fn format_chrono(d: &crate::types::DateTimeSpec, dialect: Dialect) -> String {
        use chrono::{FixedOffset, TimeZone};
        let base = FixedOffset::east_opt(7200)
            .unwrap()
            .with_ymd_and_hms(2018, 3, 21, 11, 00, 00)
            .unwrap();
        match crate::into_date_string(d.clone(), base, dialect) {
            Err(e) => {
                panic!("unexpected error attempting to format [chrono] {d:?}\n\t{e:?}")
            }
            Ok(date) => date.format("%+").to_string(),
        }
    }

    #[cfg(feature = "time_0_3")]
    #[track_caller]
    fn format_time(d: &crate::types::DateTimeSpec, dialect: Dialect) -> String {
        use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};
        let base = PrimitiveDateTime::new(
            Date::from_calendar_date(2018, Month::March, 21).unwrap(),
            Time::from_hms(11, 00, 00).unwrap(),
        )
        .assume_offset(UtcOffset::from_whole_seconds(7200).unwrap());
        match crate::into_date_string(d.clone(), base, dialect) {
            Err(e) => {
                panic!("unexpected error attempting to format [time] {d:?}\n\t{e:?}")
            }
            Ok(date) => {
                let format = time::format_description::parse(
                    "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]",
                ).unwrap();
                date.format(&format).unwrap()
            }
        }
    }

    #[cfg(feature = "jiff_0_1")]
    #[track_caller]
    fn format_jiff(d: &crate::types::DateTimeSpec, dialect: Dialect) -> String {
        use jiff::{civil::Date, civil::DateTime, civil::Time, tz::Offset, tz::TimeZone, Zoned};
        let tz = TimeZone::fixed(Offset::from_seconds(7200).unwrap());
        let base = DateTime::from_parts(Date::constant(2018, 3, 21), Time::constant(11, 00, 00, 0));
        let base = tz.to_zoned(base).unwrap();
        match crate::into_date_string(d.clone(), base, dialect) {
            Err(e) => {
                panic!("unexpected error attempting to format [time] {d:?}\n\t{e:?}")
            }
            Ok(date) => {
                // let format = time::format_description::parse(
                //     "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]",
                // ).unwrap();
                date.strftime("%FT%T%:z").to_string()
            }
        }
    }

    macro_rules! assert_date_string {
        ($s:literal, $dialect:ident, $expect:literal) => {
            let dialect = Dialect::$dialect;
            let input = $s;
            let _date = match crate::parser::DateParser::new(input).parse(dialect) {
                Err(e) => {
                    panic!("unexpected error attempting to parse [chrono] {input:?}\n\t{e:?}")
                }
                Ok(date) => date,
            };
            #[cfg(feature = "chrono_0_4")]
            {
                let output = format_chrono(&_date, dialect);
                let expected: &str = $expect;
                if output != expected {
                    panic!("unexpected output attempting to format [chrono] {input:?}.\nexpected: {expected:?}\n  parsed: {_date:?} [{output:?}]");
                }
            }
            #[cfg(feature = "time_0_3")]
            {
                let output = format_time(&_date, dialect);
                let expected: &str = $expect;
                if output != expected {
                    panic!("unexpected output attempting to format [time] {input:?}.\nexpected: {expected:?}\n  parsed: {_date:?} [{output:?}]");
                }
            }
            #[cfg(feature = "jiff_0_1")]
            {
                let output = format_jiff(&_date, dialect);
                let expected: &str = $expect;
                if output != expected {
                    panic!("unexpected output attempting to format [jiff] {input:?}.\nexpected: {expected:?}\n  parsed: {_date:?} [{output:?}]");
                }
            }
        };
    }

    #[test]
    fn basics() {
        // Day of week - relative to today. May have a time part
        assert_date_string!("friday", Uk, "2018-03-23T00:00:00+02:00");
        assert_date_string!("friday 10:30", Uk, "2018-03-23T10:30:00+02:00");
        assert_date_string!("friday 8pm", Uk, "2018-03-23T20:00:00+02:00");
        assert_date_string!("12am", Uk, "2018-03-21T00:00:00+02:00");
        assert_date_string!("12pm", Uk, "2018-03-21T12:00:00+02:00");
        assert_date_string!("7:26 AM", Uk, "2018-03-21T07:26:00+02:00");
        assert_date_string!("7:26 PM", Uk, "2018-03-21T19:26:00+02:00");

        // The day of week is the _next_ day after today, so "Tuesday" is the next Tuesday after Wednesday
        assert_date_string!("tues", Uk, "2018-03-27T00:00:00+02:00");

        // The expression 'next Monday' is ambiguous; in the US it means the day following (same as 'Monday')
        // (This is how the `date` command interprets it)
        assert_date_string!("next mon", Us, "2018-03-26T00:00:00+02:00");
        // but otherwise it means the day in the next week..
        assert_date_string!("next mon", Uk, "2018-04-02T00:00:00+02:00");

        assert_date_string!("last year", Uk, "2017-03-21T00:00:00+02:00");
        assert_date_string!("this year", Uk, "2018-03-21T00:00:00+02:00");
        assert_date_string!("next year", Uk, "2019-03-21T00:00:00+02:00");

        assert_date_string!("last fri 9.30", Uk, "2018-03-16T09:30:00+02:00");

        // date expressed as month, day - relative to today. May have a time part
        assert_date_string!("8/11", Us, "2018-08-11T00:00:00+02:00");
        assert_date_string!("last 8/11", Us, "2017-08-11T00:00:00+02:00");
        assert_date_string!("last 8/11 9am", Us, "2017-08-11T09:00:00+02:00");
        assert_date_string!("8/11", Uk, "2018-11-08T00:00:00+02:00");
        assert_date_string!("last 8/11", Uk, "2017-11-08T00:00:00+02:00");
        assert_date_string!("last 8/11 9am", Uk, "2017-11-08T09:00:00+02:00");
        assert_date_string!("April 1 8.30pm", Uk, "2018-04-01T20:30:00+02:00");

        // advance by time unit from today
        // without explicit time, use base time - otherwise override
        assert_date_string!("2d", Uk, "2018-03-23T11:00:00+02:00");
        assert_date_string!("2d 03:00", Uk, "2018-03-23T03:00:00+02:00");
        assert_date_string!("3 weeks", Uk, "2018-04-11T11:00:00+02:00");
        assert_date_string!("3h", Uk, "2018-03-21T14:00:00+02:00");
        assert_date_string!("6 months", Uk, "2018-09-21T00:00:00+02:00");
        assert_date_string!("6 months ago", Uk, "2017-09-21T00:00:00+02:00");
        assert_date_string!("3 hours ago", Uk, "2018-03-21T08:00:00+02:00");
        assert_date_string!(" -3h", Uk, "2018-03-21T08:00:00+02:00");
        assert_date_string!(" -3 month", Uk, "2017-12-21T00:00:00+02:00");

        // absolute date with year, month, day - formal ISO and informal UK or US
        assert_date_string!("2017-06-30", Uk, "2017-06-30T00:00:00+02:00");
        assert_date_string!("30/06/17", Uk, "2017-06-30T00:00:00+02:00");
        assert_date_string!("06/30/17", Us, "2017-06-30T00:00:00+02:00");

        // may be followed by time part, formal and informal
        assert_date_string!("2017-06-30 08:20:30", Uk, "2017-06-30T08:20:30+02:00");
        assert_date_string!(
            "2017-06-30 08:20:30 +04:00",
            Uk,
            "2017-06-30T06:20:30+02:00"
        );
        assert_date_string!("2017-06-30 08:20:30 +0400", Uk, "2017-06-30T06:20:30+02:00");
        assert_date_string!("2017-06-30T08:20:30Z", Uk, "2017-06-30T10:20:30+02:00");
        assert_date_string!("2017-06-30T08:20:30", Uk, "2017-06-30T08:20:30+02:00");
        assert_date_string!("2017-06-30 12.20", Uk, "2017-06-30T12:20:00+02:00");
        assert_date_string!("2017-06-30 8.20", Uk, "2017-06-30T08:20:00+02:00");
        assert_date_string!("2017-06-30 12.15am", Uk, "2017-06-30T00:15:00+02:00");
        assert_date_string!("2017-06-30 12.25pm", Uk, "2017-06-30T12:25:00+02:00");
        assert_date_string!("2017-06-30 12:30pm", Uk, "2017-06-30T12:30:00+02:00");
        assert_date_string!("2017-06-30 8.30pm", Uk, "2017-06-30T20:30:00+02:00");
        assert_date_string!("2017-06-30 8:30pm", Uk, "2017-06-30T20:30:00+02:00");
        assert_date_string!("2017-06-30 2am", Uk, "2017-06-30T02:00:00+02:00");
        assert_date_string!("30 June 2018", Uk, "2018-06-30T00:00:00+02:00");
        assert_date_string!("June 30, 2018", Uk, "2018-06-30T00:00:00+02:00");
        assert_date_string!("June   30,    2018", Uk, "2018-06-30T00:00:00+02:00");
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
            ($s:literal, $expect:expr) => {
                let err = parse_duration($s).unwrap_err();
                assert_eq!(err, $expect);
            };
        }

        assert_duration!("1 seconds", Interval::Seconds(1));
        assert_duration!("24 seconds", Interval::Seconds(24));
        assert_duration!("34 s", Interval::Seconds(34));
        assert_duration!("34 sec", Interval::Seconds(34));

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
        assert_duration_err!("2020-01-01", DateError::UnexpectedAbsoluteDate);
        assert_duration_err!("2 days 15:00", DateError::UnexpectedTime);
        assert_duration_err!("tuesday", DateError::UnexpectedDate);
        assert_duration_err!(
            "bananas",
            DateError::ExpectedToken("unsupported identifier", 0..7)
        );
    }

    #[cfg(feature = "chrono_0_4")]
    #[test]
    /// <https://github.com/conradludgate/interim/issues/12>
    fn regression_12_chrono() {
        use chrono::TimeZone;

        let now: chrono::DateTime<_> = chrono_tz::America::Los_Angeles
            .with_ymd_and_hms(2024, 1, 1, 12, 00, 00)
            .unwrap();
        let without_timezone =
            crate::parse_date_string("2024-06-01 12:00:00", now, Dialect::Us).unwrap();
        let with_timezone =
            crate::parse_date_string("2024-06-01 12:00:00 -07:00", now, Dialect::Us).unwrap();

        assert_eq!(without_timezone, with_timezone);
    }

    #[cfg(feature = "jiff_0_1")]
    #[test]
    fn regression_12_jiff() {
        use jiff::{civil::Date, civil::DateTime, civil::Time, tz::Offset, tz::TimeZone, Zoned};

        let tz = TimeZone::get("America/Los_Angeles").unwrap();
        let base = DateTime::from_parts(Date::constant(2024, 1, 1), Time::constant(12, 00, 00, 0));
        let now = tz.to_zoned(base).unwrap();

        let without_timezone =
            crate::parse_date_string("2024-06-01 12:00:00", now.clone(), Dialect::Us).unwrap();
        let with_timezone =
            crate::parse_date_string("2024-06-01 12:00:00 -07:00", now, Dialect::Us).unwrap();

        assert_eq!(without_timezone, with_timezone);
    }
}
