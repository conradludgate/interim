# interim

[![Tests](https://img.shields.io/github/actions/workflow/status/conradludgate/interim/test.yml?style=flat-square
)](https://github.com/conradludgate/interim/actions/workflows/test.yml)
[![docs](https://img.shields.io/docsrs/interim/latest?style=flat-square)](https://docs.rs/interim/latest/interim/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/interim?style=flat-square)](https://crates.io/crates/interim)

interim started as a fork, but ended up being a complete over-haul of [chrono-english](https://github.com/stevedonovan/chrono-english).

The API surface is the same, although there's some key differences

## Improvements

Why use interim over chrono-english?

1. chrono-english is not actively maintained: https://github.com/stevedonovan/chrono-english/issues/22
2. interim simplifies a lot of the code, removing a lot of potential panics and adds some optimisations.
3. supports `no_std`, as well as the `time` crate

## Features

- `std`: This crate is `no_std` compatible. Disable the default-features to disable the std-lib features (just error reporting)
- `time`: This crate is compatible with the [time crate](https://github.com/time-rs/time).
- `chrono`: This crate is compatible with the [chrono crate](https://github.com/chronotope/chrono).

## Supported Formats

`interim` does _absolute_ dates: ISO-like dates "2018-04-01" and the month name forms
"1 April 2018" and "April 1, 2018". (There's no ambiguity so both of these forms are fine)

The informal "01/04/18" or American form "04/01/18" is supported.
There is a `Dialect` enum to specify what kind of date English you would like to speak.
Both short and long years are accepted in this form; short dates pivot between 1940 and 2040.

Then there are are _relative_ dates like 'April 1' and '9/11' (this
if using `Dialect::Us`). The current year is assumed, but this can be modified by 'next'
and 'last'. For instance, it is now the 13th of March, 2018: 'April 1' and 'next April 1'
are in 2018; 'last April 1' is in 2017.

Another relative form is simply a month name
like 'apr' or 'April' (case-insensitive, only first three letters significant) where the
day is assumed to be the 1st.

A week-day works in the same way: 'friday' means this
coming Friday, relative to today. 'last Friday' is unambiguous,
but 'next Friday' has different meanings; in the US it means the same as 'Friday'
but otherwise it means the Friday of next week (plus 7 days)

Date and time can be specified also by a number of time units. So "2 days", "3 hours".
Again, first three letters, but 'd','m' and 'y' are understood (so "3h"). We make
a distinction between _second_ intervals (seconds,minutes,hours), _day_ intervals (days,weeks)
and _month_ intervals (months,years).

Second intervals are not followed by a time, but day and month intervals can be. Without
a time, a day interval has the same time as the base time (which defaults to 'now')

Month intervals always give us the same date, if possible
But adding a month to "30 Jan" will give "28 Feb" or "29 Feb" depending if a leap year.

Finally, dates may be followed by time. Either 'formal' like 18:03, with optional
second (like 18:03:40) or 'informal' like 6.03pm. So one gets "next friday 8pm' and so
forth.

## API

There are two entry points: `parse_date_string` and `parse_duration`. The
first is given the date string, a `DateTime` from which relative dates and
times operate, and a dialect (either `Dialect::Uk` or `Dialect::Us`
currently.) The base time also specifies the desired timezone.

```rust
use interim::{parse_date_string, Dialect};
use chrono::Local;

let date_time = parse_date_string("next friday 8pm", Local::now(), Dialect::Uk)?;
println!("{}", date_time.format("%c"));
```

There is a little command-line program `parse-date` in the `examples` folder which can be used to play
with these expressions.

The other function, `parse_duration`, lets you access just the relative part
of a string like 'two days ago' or '12 hours'. If successful, returns an
`Interval`, which is a number of seconds, days, or months.

```rust
use interim::{parse_duration, Interval};

assert_eq!(parse_duration("15m ago").unwrap(), Interval::Seconds(-15 * 60));
```

You can test out the library by using the CLI example,

```bash
cargo run --example cli --features time 'next day'
```
