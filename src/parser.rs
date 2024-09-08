use logos::{Lexer, Logos};

use crate::{
    types::{
        month_name, time_unit, week_day, AbsDate, ByName, DateSpec, DateTimeSpec, Direction,
        TimeSpec,
    },
    DateError, DateResult, Dialect, Interval,
};

// when we parse dates, there's often a bit of time parsed..
#[derive(Clone, Copy, Debug)]
enum TimeKind {
    Formal,
    Informal,
    Am,
    Pm,
    Unknown,
}

pub struct DateParser<'a> {
    s: Lexer<'a, Tokens>,
    maybe_time: Option<(u32, TimeKind)>,
}

#[derive(logos::Logos, Debug, PartialEq, Eq, Clone, Copy)]
#[logos(skip r"[ \t\n\f]+")]
enum Tokens {
    #[regex("[0-9]{1,4}", |lex| lex.slice().parse().map_err(|_| ()))]
    Number(u32),

    #[regex("[a-zA-Z]+")]
    Ident,

    // punctuation
    #[token("-")]
    Dash,
    #[token("/")]
    Slash,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token("+")]
    Plus,
}

impl<'a> DateParser<'a> {
    pub fn new(text: &'a str) -> DateParser<'a> {
        DateParser {
            s: Tokens::lexer(text),
            maybe_time: None,
        }
    }

    fn next_num(&mut self) -> DateResult<u32> {
        match self.s.next() {
            Some(Ok(Tokens::Number(n))) => Ok(n),
            Some(_) => Err(DateError::ExpectedToken("number", self.s.span())),
            None => Err(DateError::EndOfText("number")),
        }
    }

    fn iso_date(&mut self, year: i32) -> DateResult<DateSpec> {
        let month = self.next_num()?;

        match self.s.next() {
            Some(Ok(Tokens::Dash)) => {}
            Some(_) => return Err(DateError::ExpectedToken("'-'", self.s.span())),
            None => return Err(DateError::EndOfText("'-'")),
        }

        let day = self.next_num()?;
        Ok(DateSpec::Absolute(AbsDate { year, month, day }))
    }

    // We have already parsed maybe the next/last/...
    // and the first set of numbers followed by the slash
    //
    // US:
    // mm/dd/yy
    // mm/dd/yyyy
    // next mm/dd
    //
    // UK:
    // dd/mm/yy
    // dd/mm/yyyy
    // next dd/mm
    fn informal_date(
        &mut self,
        day_or_month: u32,
        dialect: Dialect,
        direct: Direction,
    ) -> DateResult<DateSpec> {
        let month_or_day = self.next_num()?;
        let (day, month) = if dialect == Dialect::Us {
            (month_or_day, day_or_month)
        } else {
            (day_or_month, month_or_day)
        };
        let s = self.s.clone();
        if self.s.next() != Some(Ok(Tokens::Slash)) {
            // backtrack
            self.s = s;
            Ok(DateSpec::FromName(ByName::DayMonth { day, month }, direct))
        } else {
            // pivot (1940, 2040)
            let year = match self.next_num()? as i32 {
                y @ 0..=40 => 2000 + y,
                y @ 41..=99 => 1900 + y,
                y => y,
            };
            Ok(DateSpec::Absolute(AbsDate { year, month, day }))
        }
    }

    fn parse_date(&mut self, dialect: Dialect) -> DateResult<Option<DateSpec>> {
        let (sign, direct);
        let token = match self.s.next() {
            Some(Ok(Tokens::Dash)) => {
                sign = true;
                direct = None;
                self.s.next()
            }
            Some(Ok(Tokens::Ident)) => {
                sign = false;
                direct = match self.s.slice() {
                    "now" | "today" => return Ok(Some(DateSpec::Relative(Interval::Days(0)))),
                    "yesterday" => return Ok(Some(DateSpec::Relative(Interval::Days(-1)))),
                    "tomorrow" => return Ok(Some(DateSpec::Relative(Interval::Days(1)))),
                    "next" => Some(Direction::Next),
                    "last" => Some(Direction::Last),
                    "this" => Some(Direction::Here),
                    _ => None,
                };
                if direct.is_some() {
                    // consume
                    self.s.next()
                } else {
                    Some(Ok(Tokens::Ident))
                }
            }
            t => {
                sign = false;
                direct = None;
                t
            }
        };

        match token {
            // date needs some token
            None => Err(DateError::EndOfText("empty date string")),
            // none of these characters begin a date or duration
            Some(
                Ok(
                    Tokens::Colon
                    | Tokens::Comma
                    | Tokens::Dash
                    | Tokens::Dot
                    | Tokens::Slash
                    | Tokens::Plus,
                )
                | Err(()),
            ) => Err(DateError::MissingDate),
            // '-June' doesn't make sense
            Some(Ok(Tokens::Ident)) if sign => {
                Err(DateError::ExpectedToken("number", self.s.span()))
            }
            // {weekday} [{time}]
            // {month} [{day}, {year}] [{time}]
            // {month} [{day}] [{time}]
            Some(Ok(Tokens::Ident)) => {
                let direct = direct.unwrap_or(Direction::Here);
                if let Some(month) = month_name(self.s.slice()) {
                    // {month} [{day}, {year}]
                    // {month} [{day}] [{time}]
                    if let Some(Ok(Tokens::Number(day))) = self.s.next() {
                        let s = self.s.clone();
                        if self.s.next() == Some(Ok(Tokens::Comma)) {
                            // comma found, expect year
                            let year = self.next_num()? as i32;
                            Ok(Some(DateSpec::Absolute(AbsDate { year, month, day })))
                        } else {
                            // no comma found, we might expect a time component (if any)
                            // backtrack, we'll try parse the time component later
                            self.s = s;
                            Ok(Some(DateSpec::FromName(
                                ByName::DayMonth { day, month },
                                direct,
                            )))
                        }
                    } else {
                        // We only have a month name to work with
                        Ok(Some(DateSpec::FromName(ByName::MonthName(month), direct)))
                    }
                } else if let Some(weekday) = week_day(self.s.slice()) {
                    // {weekday} [{time}]
                    // we'll try parse the time component later
                    Ok(Some(DateSpec::FromName(ByName::WeekDay(weekday), direct)))
                } else if let Some(interval) = time_unit(self.s.slice()) {
                    let interval = match direct {
                        Direction::Last => interval * -1,
                        #[allow(clippy::erasing_op)]
                        Direction::Here => interval * 0,
                        Direction::Next => interval,
                    };
                    Ok(Some(DateSpec::Relative(interval)))
                } else {
                    Err(DateError::ExpectedToken(
                        "unsupported identifier",
                        self.s.span(),
                    ))
                }
            }
            // {day}/{month}
            // {month}/{day}
            // {day} {month}
            // {n} {interval}
            // {year}-{month}-{day}
            Some(Ok(Tokens::Number(n))) => {
                match self.s.next() {
                    // if sign is set, we should expect something like '- 5 minutes'
                    None if sign => Err(DateError::EndOfText("duration")),
                    // we want a full date
                    Some(Ok(Tokens::Comma | Tokens::Plus | Tokens::Number(_)) | Err(())) => {
                        Err(DateError::ExpectedToken("date", self.s.span()))
                    }
                    // if direct is set, we should expect a day or month to direct against
                    None | Some(Ok(Tokens::Colon | Tokens::Dot | Tokens::Dash))
                        if direct.is_some() =>
                    {
                        Err(DateError::EndOfText("day or month name"))
                    }
                    // if no extra tokens, this is probably just a year
                    None => Ok(Some(DateSpec::Absolute(AbsDate {
                        year: n as i32,
                        month: 1,
                        day: 1,
                    }))),
                    Some(Ok(Tokens::Ident)) => {
                        let direct = direct.unwrap_or(Direction::Here);
                        let name = self.s.slice();
                        if let Some(month) = month_name(name) {
                            let day = n;
                            if let Some(Ok(Tokens::Number(year))) = self.s.next() {
                                // 4 July 2017
                                let year = year as i32;
                                Ok(Some(DateSpec::Absolute(AbsDate { year, month, day })))
                            } else {
                                // 4 July
                                Ok(Some(DateSpec::FromName(
                                    ByName::DayMonth { day, month },
                                    direct,
                                )))
                            }
                        } else if let Some(u) = time_unit(name) {
                            let n = n as i32;
                            // '2 days'
                            if sign {
                                Ok(Some(DateSpec::Relative(u * -n)))
                            } else {
                                match self.s.next() {
                                    Some(Ok(Tokens::Ident)) if self.s.slice() == "ago" => {
                                        Ok(Some(DateSpec::Relative(u * -n)))
                                    }
                                    Some(Ok(Tokens::Ident)) => {
                                        Err(DateError::ExpectedToken("'ago'", self.s.span()))
                                    }
                                    Some(Ok(Tokens::Number(h))) => {
                                        self.maybe_time = Some((h, TimeKind::Unknown));
                                        Ok(Some(DateSpec::Relative(u * n)))
                                    }
                                    _ => Ok(Some(DateSpec::Relative(u * n))),
                                }
                            }
                        } else if name == "am" {
                            self.maybe_time = Some((n, TimeKind::Am));
                            Ok(None)
                        } else if name == "pm" {
                            self.maybe_time = Some((n, TimeKind::Pm));
                            Ok(None)
                        } else {
                            Err(DateError::ExpectedToken(
                                "month or time unit",
                                self.s.span(),
                            ))
                        }
                    }
                    Some(Ok(Tokens::Colon)) => {
                        self.maybe_time = Some((n, TimeKind::Formal));
                        Ok(None)
                    }
                    Some(Ok(Tokens::Dot)) => {
                        self.maybe_time = Some((n, TimeKind::Informal));
                        Ok(None)
                    }
                    Some(Ok(Tokens::Dash)) => Ok(Some(self.iso_date(n as i32)?)),
                    Some(Ok(Tokens::Slash)) => Ok(Some(self.informal_date(
                        n,
                        dialect,
                        direct.unwrap_or(Direction::Here),
                    )?)),
                }
            }
        }
    }

    fn formal_time(&mut self, hour: u32) -> DateResult<TimeSpec> {
        let min = self.next_num()?;
        let mut sec = 0;
        let mut micros = 0;

        // minute may be followed by [:secs][am|pm]
        let tnext = match self.s.next() {
            Some(Ok(Tokens::Colon)) => {
                sec = self.next_num()?;
                match self.s.next() {
                    Some(Ok(Tokens::Dot)) => {
                        // after a `.` implies these are subseconds.
                        // We only care for microsecond precision, so let's
                        // get only the 6 most significant digits
                        micros = self.next_num()?;
                        while micros > 1_000_000 {
                            micros /= 10;
                        }
                        self.s.next()
                    }
                    t => t,
                }
            }
            // we don't expect any of these after parsing minutes
            Some(
                Ok(Tokens::Dash | Tokens::Slash | Tokens::Dot | Tokens::Comma | Tokens::Plus)
                | Err(()),
            ) => {
                return Err(DateError::ExpectedToken("':'", self.s.span()));
            }
            t => t,
        };

        match tnext {
            // we need no timezone or hour offset. All good :)
            None => Ok(TimeSpec::new(hour, min, sec, micros)),
            // +/- timezone offset
            Some(Ok(tok @ (Tokens::Plus | Tokens::Dash))) => {
                let sign = if tok == Tokens::Dash { -1 } else { 1 };

                // after a +/-, we expect a numerical offset.
                // either HH:MM or HHMM
                let mut hours = self.next_num()?;

                let s = self.s.clone();
                let minutes = if self.s.next() != Some(Ok(Tokens::Colon)) {
                    // backtrack, we should have the hours and minutes in the single number
                    self.s = s;

                    // 0030
                    //   ^^
                    let minutes = hours % 100;
                    hours /= 100;
                    minutes
                } else {
                    // 02:00
                    //    ^^
                    self.next_num()?
                };
                // hours and minutes offset in seconds
                let res = 60 * (minutes + 60 * hours);
                let offset = i64::from(res) * sign;
                Ok(TimeSpec::new(hour, min, sec, micros).with_offset(offset))
            }
            Some(Ok(Tokens::Ident)) => match self.s.slice() {
                // 0-offset timezone
                "Z" => Ok(TimeSpec::new(hour, min, sec, micros).with_offset(0)),
                // morning
                "am" if hour == 12 => Ok(TimeSpec::new(0, min, sec, micros)),
                "am" => Ok(TimeSpec::new(hour, min, sec, micros)),
                // afternoon
                "pm" if hour == 12 => Ok(TimeSpec::new(12, min, sec, micros)),
                "pm" => Ok(TimeSpec::new(hour + 12, min, sec, micros)),
                _ => Err(DateError::ExpectedToken("expected Z/am/pm", self.s.span())),
            },
            Some(
                Ok(Tokens::Slash | Tokens::Colon | Tokens::Dot | Tokens::Comma | Tokens::Number(_))
                | Err(()),
            ) => Err(DateError::ExpectedToken("expected timezone", self.s.span())),
        }
    }

    fn informal_time(&mut self, hour: u32) -> DateResult<TimeSpec> {
        let min = self.next_num()?;

        let hour = match self.s.next() {
            None => hour,
            Some(Ok(Tokens::Ident)) if self.s.slice() == "am" && hour == 12 => 0,
            Some(Ok(Tokens::Ident)) if self.s.slice() == "am" => hour,
            Some(Ok(Tokens::Ident)) if self.s.slice() == "pm" && hour == 12 => 12,
            Some(Ok(Tokens::Ident)) if self.s.slice() == "pm" => hour + 12,
            Some(_) => return Err(DateError::ExpectedToken("expected am/pm", self.s.span())),
        };

        Ok(TimeSpec::new(hour, min, 0, 0))
    }

    pub fn parse_time(&mut self) -> DateResult<Option<TimeSpec>> {
        // here the date parser looked ahead and saw an hour followed by some separator
        if let Some((h, kind)) = self.maybe_time {
            Ok(Some(match kind {
                TimeKind::Formal => self.formal_time(h)?,
                TimeKind::Informal => self.informal_time(h)?,
                TimeKind::Am if h == 12 => TimeSpec::new(0, 0, 0, 0),
                TimeKind::Am => TimeSpec::new(h, 0, 0, 0),
                TimeKind::Pm if h == 12 => TimeSpec::new(12, 0, 0, 0),
                TimeKind::Pm => TimeSpec::new(h + 12, 0, 0, 0),
                TimeKind::Unknown => match self.s.next() {
                    Some(Ok(Tokens::Colon)) => self.formal_time(h)?,
                    Some(Ok(Tokens::Dot)) => self.informal_time(h)?,
                    Some(_) => return Err(DateError::ExpectedToken(": or .", self.s.span())),
                    None => return Err(DateError::EndOfText(": or .")),
                },
            }))
        } else {
            let s = self.s.clone();
            if self.s.next() != Some(Ok(Tokens::Ident)) || self.s.slice() != "T" {
                // backtrack if we weren't able to consume a 'T' time separator
                self.s = s;
            }

            // we're parsing times so we should expect an hour number.
            // if we don't find one, then there's no time here
            let hour = match self.s.next() {
                None => return Ok(None),
                Some(Ok(Tokens::Number(n))) => n,
                Some(_) => return Err(DateError::ExpectedToken("number", self.s.span())),
            };

            match self.s.next() {
                // hh:mm
                Some(Ok(Tokens::Colon)) => self.formal_time(hour).map(Some),
                // hh.mm
                Some(Ok(Tokens::Dot)) => self.informal_time(hour).map(Some),
                // 9am
                Some(Ok(Tokens::Ident)) => match self.s.slice() {
                    "am" => Ok(Some(TimeSpec::new(hour, 0, 0, 0))),
                    "pm" => Ok(Some(TimeSpec::new(hour + 12, 0, 0, 0))),
                    _ => Err(DateError::ExpectedToken("am/pm", self.s.span())),
                },
                Some(_) => Err(DateError::ExpectedToken("am/pm, ':' or '.'", self.s.span())),
                None => Err(DateError::EndOfText("am/pm, ':' or '.'")),
            }
        }
    }

    pub fn parse(&mut self, dialect: Dialect) -> DateResult<DateTimeSpec> {
        let date = self.parse_date(dialect)?;
        let time = self.parse_time()?;
        Ok(DateTimeSpec { date, time })
    }
}
