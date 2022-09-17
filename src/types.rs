use chrono::prelude::*;
use chrono::Duration;

use crate::Dialect;

// implements next/last direction in expressions like 'next friday' and 'last 4 july'
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Next,
    Last,
    Here,
}

// this is a day-month with direction, like 'next 10 Dec'
#[derive(Debug)]
pub struct YearDate {
    pub month: u32,
    pub day: u32,
}

// all expressions modifiable with next/last; 'fri', 'jul', '5 may'.
#[derive(Debug)]
pub enum ByName {
    WeekDay(Weekday),
    MonthName(u32),
    DayMonth { day: u32, month: u32 },
}

fn add_days<Tz: TimeZone>(base: DateTime<Tz>, days: i64) -> Option<DateTime<Tz>> {
    base.checked_add_signed(Duration::days(days))
}

//fn next_last_direction<Tz: TimeZone>(date: Date<Tz>, base: Date<Tz>, direct: Direction) -> Option<i32> {

fn next_last_direction<T: PartialOrd>(date: &T, base: &T, direct: Direction) -> Option<i32> {
    match (date.partial_cmp(base), direct) {
        (Some(core::cmp::Ordering::Greater), Direction::Last) => Some(-1),
        (Some(core::cmp::Ordering::Less), Direction::Next) => Some(1),
        _ => None,
    }
}

impl ByName {
    pub fn into_date_time<Tz: TimeZone>(
        self,
        base: &DateTime<Tz>,
        ts: Option<TimeSpec>,
        dialect: Dialect,
        mut direct: Direction,
    ) -> Option<DateTime<Tz>> {
        let ts = ts.unwrap_or(TimeSpec {
            hour: 0,
            min: 0,
            sec: 0,
            microsec: 0,
            offset: None,
        });
        let this_year = base.year();
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
                let this_day = base.weekday().num_days_from_monday() as i64;
                let that_day = nd as i64;
                let diff_days = that_day - this_day;
                let mut date = add_days(base.clone(), diff_days)?;
                if let Some(correct) = next_last_direction(&date, base, direct) {
                    date = add_days(date, 7 * correct as i64)?;
                }
                if extra_week {
                    date = add_days(date, 7)?;
                }
                if diff_days == 0 {
                    // same day - comparing times will determine which way we swing...
                    let base_time = base.time();
                    let this_time = NaiveTime::from_hms(ts.hour, ts.min, ts.sec);
                    if let Some(correct) = next_last_direction(&this_time, &base_time, direct) {
                        date = add_days(date, 7 * correct as i64)?;
                    }
                }
                date.date()
            }
            ByName::MonthName(month) => {
                let mut date = base.timezone().ymd_opt(this_year, month, 1).single()?;
                if let Some(correct) = next_last_direction(&date, &base.date(), direct) {
                    date = base
                        .timezone()
                        .ymd_opt(this_year + correct, month, 1)
                        .single()?;
                }
                date
            }
            ByName::DayMonth { day, month } => {
                let mut date = base.timezone().ymd_opt(this_year, month, day).single()?;
                if let Some(correct) = next_last_direction(&date, &base.date(), direct) {
                    date = base
                        .timezone()
                        .ymd_opt(this_year + correct, month, day)
                        .single()?;
                }
                date
            }
        };
        Some(ts.into_date_time(&date))
    }
}

#[derive(Debug)]
pub struct AbsDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl AbsDate {
    pub fn into_date<Tz: TimeZone>(self, base: &DateTime<Tz>) -> Option<Date<Tz>> {
        base.timezone()
            .ymd_opt(self.year, self.month, self.day)
            .single()
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
#[derive(Debug, PartialEq, Eq)]
pub enum Interval {
    Seconds(i32),
    Days(i32),
    Months(i32),
}

#[derive(Debug)]
pub struct Skip {
    pub unit: Interval,
    pub skip: i32,
}

impl Skip {
    pub fn into_date_time<Tz: TimeZone>(
        self,
        base: DateTime<Tz>,
        ts: Option<TimeSpec>,
    ) -> Option<DateTime<Tz>> {
        match self.into_interval() {
            Interval::Seconds(secs) => {
                let dur = Duration::seconds(secs as i64);
                base.checked_add_signed(dur)
            }
            Interval::Days(days) => {
                let secs = 60 * 60 * 24 * days;
                let dur = Duration::seconds(secs as i64);
                let date = base.checked_add_signed(dur)?;
                if let Some(ts) = ts {
                    Some(ts.into_date_time(&date.date()))
                } else {
                    Some(date)
                }
            }
            Interval::Months(months) => {
                let d = base.naive_local().date();
                let date = if months >= 0 {
                    d.checked_add_months(chrono::Months::new(months as u32))?
                } else {
                    d.checked_sub_months(chrono::Months::new(-months as u32))?
                };
                let date = base.timezone().from_local_date(&date).single()?;

                if let Some(ts) = ts {
                    Some(ts.into_date_time(&date))
                } else {
                    Some(date.and_hms(0, 0, 0))
                }
            }
        }
    }

    pub fn into_interval(self) -> Interval {
        match self.unit {
            Interval::Seconds(s) => Interval::Seconds(s * self.skip),
            Interval::Days(d) => Interval::Days(d * self.skip),
            Interval::Months(m) => Interval::Months(m * self.skip),
        }
    }
}

#[derive(Debug)]
pub enum DateSpec {
    Absolute(AbsDate),           // Y M D (e.g. 2018-06-02, 4 July 2017)
    Relative(Skip),              // n U (e.g. 2min, 3 years ago, -2d)
    FromName(ByName, Direction), // (e.g. 'next fri', 'jul')
}

impl DateSpec {
    pub fn absolute(y: u32, m: u32, d: u32) -> DateSpec {
        DateSpec::Absolute(AbsDate {
            year: y as i32,
            month: m,
            day: d,
        })
    }

    pub fn from_day_month(day: u32, month: u32, direct: Direction) -> DateSpec {
        DateSpec::FromName(ByName::DayMonth { day, month }, direct)
    }

    pub fn skip(unit: Interval, n: i32) -> DateSpec {
        DateSpec::Relative(Skip { unit, skip: n })
    }

    pub fn into_date_time<Tz: TimeZone>(
        self,
        base: DateTime<Tz>,
        ts: Option<TimeSpec>,
        dialect: Dialect,
    ) -> Option<DateTime<Tz>> {
        match self {
            DateSpec::Absolute(ad) => match ts {
                Some(ts) => Some(ts.into_date_time(&ad.into_date(&base)?)),
                None => Some(ad.into_date(&base)?.and_hms(0, 0, 0)),
            },
            DateSpec::Relative(skip) => skip.into_date_time(base, ts),
            DateSpec::FromName(byname, direct) => byname.into_date_time(&base, ts, dialect, direct),
        }
    }
}

#[derive(Debug)]
pub struct TimeSpec {
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub microsec: u32,
    pub offset: Option<i64>,
}

impl TimeSpec {
    pub fn new(hour: u32, min: u32, sec: u32, microsec: u32) -> TimeSpec {
        TimeSpec {
            hour,
            min,
            sec,
            offset: None,
            microsec,
        }
    }

    pub fn new_with_offset(hour: u32, min: u32, sec: u32, offset: i64, microsec: u32) -> TimeSpec {
        TimeSpec {
            hour,
            min,
            sec,
            offset: Some(offset),
            microsec,
        }
    }

    // pub fn new_empty() -> TimeSpec {
    //     TimeSpec {
    //         hour: 0,
    //         min: 0,
    //         sec: 0,
    //         empty: true,
    //         offset: None,
    //         microsec: 0,
    //     }
    // }

    // pub fn empty(&self) -> bool {
    //     self.empty
    // }

    pub fn into_date_time<Tz: TimeZone>(self, d: &Date<Tz>) -> DateTime<Tz> {
        let dt = d.and_hms_micro(self.hour, self.min, self.sec, self.microsec);
        if let Some(offs) = self.offset {
            let zoffset = dt.offset().clone();
            let tstamp = dt.timestamp() - offs + zoffset.fix().local_minus_utc() as i64;
            let nd = NaiveDateTime::from_timestamp(tstamp, 1000 * self.microsec);
            DateTime::from_utc(nd, zoffset)
        } else {
            dt
        }
    }
}

#[derive(Debug)]
pub struct DateTimeSpec {
    pub date: Option<DateSpec>,
    pub time: Option<TimeSpec>,
}

// same as chrono's 'count days from monday' convention
pub fn week_day(s: &str) -> Option<Weekday> {
    let mut s = match s.as_bytes() {
        [a, b, c, ..] => [*a, *b, *c],
        _ => return None,
    };
    s.make_ascii_lowercase();
    match &s {
        b"sun" => Some(Weekday::Sun),
        b"mon" => Some(Weekday::Mon),
        b"tue" => Some(Weekday::Tue),
        b"wed" => Some(Weekday::Wed),
        b"thu" => Some(Weekday::Thu),
        b"fri" => Some(Weekday::Fri),
        b"sat" => Some(Weekday::Sat),
        _ => None,
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

pub fn time_unit(s: &str) -> Option<Interval> {
    let s = if s.len() > 3 { &s[..3] } else { s };
    match s.as_bytes() {
        b"sec" | b"s" => Some(Interval::Seconds(1)),
        b"min" | b"m" => Some(Interval::Seconds(60)),
        b"hou" | b"h" => Some(Interval::Seconds(60 * 60)),
        b"day" | b"d" => Some(Interval::Days(1)),
        b"wee" | b"w" => Some(Interval::Days(7)),
        b"mon" => Some(Interval::Months(1)),
        b"yea" | b"y" => Some(Interval::Months(12)),
        _ => None,
    }
}
