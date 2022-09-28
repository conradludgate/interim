use core::ops::Mul;

use crate::datetime::{Date, DateTime, Time, Timezone};
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
                };
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
            let offset = tz.local_minus_utc() - offs;
            Dt::new(tz, date, time).offset_seconds(offset)
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

// same as chrono's 'count days from monday' convention
pub fn week_day(input: &str) -> Option<u8> {
    // wednesday is the longest day
    const MAX_SIZE: usize = 9;
    if input.len() > MAX_SIZE {
        return None;
    }
    let mut buffer = [0; MAX_SIZE];
    buffer[..input.len()].copy_from_slice(input.as_bytes());
    buffer.make_ascii_lowercase();
    if buffer.starts_with(b"su") {
        Some(6)
    } else if buffer.starts_with(b"mo") {
        Some(0)
    } else if buffer.starts_with(b"tu") {
        Some(1)
    } else if buffer.starts_with(b"we") {
        Some(2)
    } else if buffer.starts_with(b"th") {
        Some(3)
    } else if buffer.starts_with(b"fr") {
        Some(4)
    } else if buffer.starts_with(b"sa") {
        Some(5)
    } else {
        None
    }
}

pub fn month_name(s: &str) -> Option<u32> {
    let mut s = match s.as_bytes() {
        [a, b, c, ..] => [*a, *b, *c],
        _ => return None,
    };
    s.make_ascii_lowercase();
    match &s {
        b"jan" => Some(1),
        b"feb" => Some(2),
        b"mar" => Some(3),
        b"apr" => Some(4),
        b"may" => Some(5),
        b"jun" => Some(6),
        b"jul" => Some(7),
        b"aug" => Some(8),
        b"sep" => Some(9),
        b"oct" => Some(10),
        b"nov" => Some(11),
        b"dec" => Some(12),
        _ => None,
    }
}

pub fn time_unit(input: &str) -> Option<Interval> {
    const MAX_SIZE: usize = 7;
    if input.len() > MAX_SIZE {
        return None;
    }
    let mut buffer = [0; MAX_SIZE];
    buffer[..input.len()].copy_from_slice(input.as_bytes());
    buffer.make_ascii_lowercase();
    if buffer[0] == 's' as u8 || buffer.starts_with(b"se") {
        Some(Interval::Seconds(1))
    } else if (buffer[0] == 'm' as u8 && input.len() == 1) || buffer.starts_with(b"mi") {
        Some(Interval::Seconds(60))
    } else if buffer[0] == 'h' as u8 || buffer.starts_with(b"ho") {
        Some(Interval::Seconds(60 * 60))
    } else if buffer[0] == 'd' as u8 || buffer.starts_with(b"da") {
        Some(Interval::Days(1))
    } else if buffer[0] == 'w' as u8 || buffer.starts_with(b"we") {
        Some(Interval::Days(7))
    } else if buffer.starts_with(b"mo") {
        Some(Interval::Months(1))
    } else if buffer[0] == 'y' as u8 || buffer.starts_with(b"ye") {
        Some(Interval::Months(12))
    } else {
        None
    }
}
