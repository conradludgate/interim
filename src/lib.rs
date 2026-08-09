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
//! * `time_0_3`: This crate is compatible with the [time crate](https://github.com/time-rs/time).
//! * `chrono_0_4`: This crate is compatible with the [chrono crate](https://github.com/chronotope/chrono).
//! * `jiff_0_1`: This crate is compatible with the v0.1 [jiff crate](https://github.com/BurntSushi/jiff).
//! * `jiff_0_2`: This crate is compatible with the v0.2 [jiff crate](https://github.com/BurntSushi/jiff).
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
