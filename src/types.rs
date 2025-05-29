use core::ops::Mul;

use crate::datetime::{Date, DateTime, Time};
use crate::Dialect;

// implements next/last direction in expressions like 'next friday' and 'last 4 july'
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Next,
    Last,
    Here,
}

// all expressions modifiable with next/last; 'fri', 'jul', '5 may'.
#[derive(Debug, Clone)]
pub enum ByName {
    WeekDay(u8),
    MonthName(u32),
    DayMonth { day: u32, month: u32 },
}

// fn add_days<Tz: TimeZone>(base: DateTime<Tz>, days: i64) -> Option<DateTime<Tz>> {
//     base.checked_add_signed(Duration::days(days))
// }

fn next_last_direction<T: PartialOrd>(date: &T, base: &T, direct: Direction) -> Option<i32> {
    match (date.partial_cmp(base), direct) {
        (Some(core::cmp::Ordering::Greater), Direction::Last) => Some(-1),
        (Some(core::cmp::Ordering::Less), Direction::Next) => Some(1),
        _ => None,
    }
}

impl ByName {
    pub fn into_date_time<Dt: DateTime>(
        self,
        base: Dt,
        ts: Option<TimeSpec>,
        dialect: Dialect,
        mut direct: Direction,
    ) -> Option<Dt> {
        let (tz, base_date, base_time) = base.split();
        let ts = ts.unwrap_or(TimeSpec::new(0, 0, 0, 0));
        let this_year = base_date.year();
        let date = match self {
            ByName::WeekDay(nd) => {
                // a plain 'Friday' means the same as 'next Friday'.
                // an _explicit_ 'next Friday' has dialect-dependent meaning!
                // In UK English, it means 'Friday of next week',
                // but in US English, just the next Friday
                let mut extra_week = false;
                match direct {
                    Direction::Here => direct = Direction::Next,
                    Direction::Next if dialect == Dialect::Uk => {
                        extra_week = true;
                    }
                    _ => (),
                }
                let this_day = base_date.weekday() as i64;
                let that_day = nd as i64;
                let diff_days = that_day - this_day;
                let mut date = base_date.clone().offset_days(diff_days)?;
                if let Some(correct) = next_last_direction(&date, &base_date, direct) {
                    date = date.offset_days(7 * correct as i64)?;
                }
                if extra_week {
                    date = date.offset_days(7)?;
                }
                if diff_days == 0 {
                    // same day - comparing times will determine which way we swing...
                    let this_time = <Dt::Time as Time>::from_hms(ts.hour, ts.min, ts.sec)?;
                    if let Some(correct) = next_last_direction(&this_time, &base_time, direct) {
                        date = date.offset_days(7 * correct as i64)?;
                    }
                }
                date
            }
            ByName::MonthName(month) => {
                let mut date = <Dt::Date as Date>::from_ymd(this_year, month as u8, 1)?;
                if let Some(correct) = next_last_direction(&date, &base_date, direct) {
                    date = <Dt::Date as Date>::from_ymd(this_year + correct, month as u8, 1)?;
                }
                date
            }
            ByName::DayMonth { day, month } => {
                let mut date = <Dt::Date as Date>::from_ymd(this_year, month as u8, day as u8)?;
                if let Some(correct) = next_last_direction(&date, &base_date, direct) {
                    date =
                        <Dt::Date as Date>::from_ymd(this_year + correct, month as u8, day as u8)?;
                }
                date
            }
        };
        ts.into_date_time(tz, date)
    }
}

#[derive(Debug, Clone)]
pub struct AbsDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl AbsDate {
    pub fn into_date<D: Date>(self) -> Option<D> {
        D::from_ymd(self.year, self.month as u8, self.day as u8)
    }
}

/// A generic amount of time, in either seconds, days, or months.
///
/// This way, a user can decide how they want to treat days (which do
/// not always have the same number of seconds) or months (which do not always
/// have the same number of days).
//
// Skipping a given number of time units.
// The subtlety is that we treat duration as seconds until we get
// to months, where we want to preserve dates. So adding a month to
// '5 May' gives '5 June'. Adding a month to '30 Jan' gives 'Feb 28' or 'Feb 29'
// depending on whether this is a leap year.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Interval {
    Seconds(i32),
    Days(i32),
    Months(i32),
}

impl Mul<i32> for Interval {
    type Output = Interval;

    fn mul(self, rhs: i32) -> Self::Output {
        match self {
            Interval::Seconds(x) => Interval::Seconds(x * rhs),
            Interval::Days(x) => Interval::Days(x * rhs),
            Interval::Months(x) => Interval::Months(x * rhs),
        }
    }
}

impl Interval {
    fn into_date_time<Dt: DateTime>(self, base: Dt, ts: Option<TimeSpec>) -> Option<Dt> {
        match self {
            Interval::Seconds(secs) => {
                // since numbers of seconds _is a timespec_, we don't add the timespec on top
                // eg now + 15m shouldn't then process 12pm after it.
                // Ideally Interval::Seconds should be part of timespec.
                base.offset_seconds(secs as i64)
            }
            Interval::Days(days) => {
                let (tz, date, time) = base.split();
                let date = date.offset_days(days as i64)?;
                if let Some(ts) = ts {
                    ts.into_date_time(tz, date)
                } else {
                    Some(Dt::new(tz, date, time))
                }
            }
            Interval::Months(months) => {
                let (tz, date, _) = base.split();
                let date = date.offset_months(months)?;
                if let Some(ts) = ts {
                    ts.into_date_time(tz, date)
                } else {
                    let time = <Dt::Time as Time>::from_hms(0, 0, 0)?;
                    Some(Dt::new(tz, date, time))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum DateSpec {
    Absolute(AbsDate),           // Y M D (e.g. 2018-06-02, 4 July 2017)
    Relative(Interval),          // n U (e.g. 2min, 3 years ago, -2d)
    FromName(ByName, Direction), // (e.g. 'next fri', 'jul')
}

impl DateSpec {
    pub fn into_date_time<Dt: DateTime>(
        self,
        base: Dt,
        ts: Option<TimeSpec>,
        dialect: Dialect,
    ) -> Option<Dt> {
        match self {
            DateSpec::Absolute(ad) => match ts {
                Some(ts) => ts.into_date_time(base.split().0, ad.into_date()?),
                None => Some(Dt::new(
                    base.split().0,
                    ad.into_date::<Dt::Date>()?,
                    <Dt::Time>::from_hms(0, 0, 0)?,
                )),
            },
            DateSpec::Relative(skip) => skip.into_date_time(base, ts),
            DateSpec::FromName(byname, direct) => byname.into_date_time(base, ts, dialect, direct),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeSpec {
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub microsec: u32,
    pub offset: Option<i64>,
}

impl TimeSpec {
    pub const fn new(hour: u32, min: u32, sec: u32, microsec: u32) -> Self {
        Self {
            hour,
            min,
            sec,
            microsec,
            offset: None,
        }
    }

    pub fn with_offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn into_date_time<Dt: DateTime>(self, tz: Dt::TimeZone, date: Dt::Date) -> Option<Dt> {
        let date = date.offset_days((self.hour / 24) as i64)?;
        let time = <Dt::Time as Time>::from_hms(self.hour % 24, self.min, self.sec)?
            .with_micros(self.microsec)?;
        if let Some(offs) = self.offset {
            Dt::new(tz, date, time).with_offset(offs)
        } else {
            Some(Dt::new(tz, date, time))
        }
    }
}

#[derive(Debug, Clone)]
pub struct DateTimeSpec {
    pub date: Option<DateSpec>,
    pub time: Option<TimeSpec>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) struct Lowercase([u8; 16]);

impl Lowercase {
    pub(crate) const fn literal(s: &str) -> Self {
        assert!(s.len() < 16);
        let mut i = 0;
        let mut out = [0; 16];
        while i < s.len() {
            assert!(s.as_bytes()[i].is_ascii_lowercase());
            out[i] = s.as_bytes()[i];
            i += 1;
        }
        Self(out)
    }

    fn truncate(mut self, n: usize) -> Self {
        self.0[n..].fill(0);
        self
    }
}

impl From<&str> for Lowercase {
    fn from(value: &str) -> Self {
        if value.len() > 16 {
            // some value that will never be equal to a literal
            return Self(*b"AAAAAAAAAAAAAAAA");
        }
        let mut out = [0; 16];
        out[..value.len()].copy_from_slice(value.as_bytes());
        out.make_ascii_lowercase();
        Self(out)
    }
}

// same as chrono's 'count days from monday' convention
pub(crate) fn week_day(s: Lowercase) -> Option<u8> {
    const SUN: Lowercase = Lowercase::literal("sun");
    const MON: Lowercase = Lowercase::literal("mon");
    const TUE: Lowercase = Lowercase::literal("tue");
    const WED: Lowercase = Lowercase::literal("wed");
    const THU: Lowercase = Lowercase::literal("thu");
    const FRI: Lowercase = Lowercase::literal("fri");
    const SAT: Lowercase = Lowercase::literal("sat");

    match s.truncate(3) {
        SUN => Some(6),
        MON => Some(0),
        TUE => Some(1),
        WED => Some(2),
        THU => Some(3),
        FRI => Some(4),
        SAT => Some(5),
        _ => None,
    }
}

pub(crate) fn month_name(s: Lowercase) -> Option<u32> {
    const JAN: Lowercase = Lowercase::literal("jan");
    const FEB: Lowercase = Lowercase::literal("feb");
    const MAR: Lowercase = Lowercase::literal("mar");
    const APR: Lowercase = Lowercase::literal("apr");
    const MAY: Lowercase = Lowercase::literal("may");
    const JUN: Lowercase = Lowercase::literal("jun");
    const JUL: Lowercase = Lowercase::literal("jul");
    const AUG: Lowercase = Lowercase::literal("aug");
    const SEP: Lowercase = Lowercase::literal("sep");
    const OCT: Lowercase = Lowercase::literal("oct");
    const NOV: Lowercase = Lowercase::literal("nov");
    const DEC: Lowercase = Lowercase::literal("dec");

    match s.truncate(3) {
        JAN => Some(1),
        FEB => Some(2),
        MAR => Some(3),
        APR => Some(4),
        MAY => Some(5),
        JUN => Some(6),
        JUL => Some(7),
        AUG => Some(8),
        SEP => Some(9),
        OCT => Some(10),
        NOV => Some(11),
        DEC => Some(12),
        _ => None,
    }
}

pub(crate) fn time_unit(input: Lowercase) -> Option<Interval> {
    if input == Lowercase::literal("s") || input.0.starts_with(b"se") {
        Some(Interval::Seconds(1))
    } else if input == Lowercase::literal("m") || input.0.starts_with(b"mi") {
        Some(Interval::Seconds(60))
    } else if input == Lowercase::literal("h") || input.0.starts_with(b"ho") {
        Some(Interval::Seconds(60 * 60))
    } else if input == Lowercase::literal("d") || input.0.starts_with(b"da") {
        Some(Interval::Days(1))
    } else if input == Lowercase::literal("w") || input.0.starts_with(b"we") {
        Some(Interval::Days(7))
    } else if input.0.starts_with(b"mo") {
        Some(Interval::Months(1))
    } else if input == Lowercase::literal("y") || input.0.starts_with(b"ye") {
        Some(Interval::Months(12))
    } else {
        None
    }
}
