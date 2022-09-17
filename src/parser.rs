use logos::{Lexer, Logos};

use crate::{
    types::{month_name, time_unit, week_day, ByName, DateSpec, DateTimeSpec, Direction, TimeSpec},
    DateError, DateResult, Dialect, Interval,
};

// when we parse dates, there's often a bit of time parsed..
#[derive(Clone, Copy, Debug)]
enum TimeKind {
    Formal,
    Informal,
    AmPm(bool),
    Unknown,
}

pub struct DateParser<'a> {
    s: Lexer<'a, Tokens>,
    maybe_time: Option<(u32, TimeKind)>,
}

#[derive(logos::Logos, Debug, PartialEq, Eq, Clone, Copy)]
enum Tokens {
    #[regex("[0-9]+", |lex| lex.slice().parse())]
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

    #[regex(r"[ \t\n\f]+", logos::skip)]
    #[error]
    Error,
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
            Some(Tokens::Number(n)) => Ok(n),
            Some(_) => Err(DateError::UnexpectedToken("number", self.s.span())),
            None => Err(DateError::UnexpectedEndOfText("number")),
        }
    }

    fn peek(&self) -> Tokens {
        self.s.clone().next().unwrap_or(Tokens::Error)
    }

    fn iso_date(&mut self, y: u32) -> DateResult<DateSpec> {
        let month = self.next_num()?;

        match self.s.next() {
            Some(Tokens::Dash) => {}
            Some(_) => return Err(DateError::UnexpectedToken("'-'", self.s.span())),
            None => return Err(DateError::UnexpectedEndOfText("'-'")),
        }

        let day = self.next_num()?;
        Ok(DateSpec::absolute(y, month, day))
    }

    fn informal_date(
        &mut self,
        day_or_month: u32,
        dialect: Dialect,
        direct: Direction,
    ) -> DateResult<DateSpec> {
        let month_or_day = self.next_num()?;
        let (d, m) = if dialect == Dialect::Us {
            (month_or_day, day_or_month)
        } else {
            (day_or_month, month_or_day)
        };
        let date = if self.peek() == Tokens::Slash {
            let _ = self.s.next();
            let y = self.next_num()?;
            let y = if y < 100 {
                // pivot (1940, 2040)
                if y > 40 {
                    1900 + y
                } else {
                    2000 + y
                }
            } else {
                y
            };
            DateSpec::absolute(y, m, d)
        } else {
            DateSpec::FromName(ByName::from_day_month(d, m), direct)
        };
        Ok(date)
    }

    fn parse_date(&mut self, dialect: Dialect) -> DateResult<Option<DateSpec>> {
        let (sign, direct);
        let token = match self.s.next() {
            Some(Tokens::Dash) => {
                sign = true;
                direct = None;
                self.s.next()
            }
            Some(Tokens::Ident) => {
                sign = false;
                direct = match self.s.slice() {
                    "now" | "today" => return Ok(Some(DateSpec::skip(Interval::Days(1), 0))),
                    "yesterday" => return Ok(Some(DateSpec::skip(Interval::Days(1), -1))),
                    "tomorrow" => return Ok(Some(DateSpec::skip(Interval::Days(1), 1))),
                    "next" => Some(Direction::Next),
                    "last" => Some(Direction::Last),
                    "this" => Some(Direction::Here),
                    _ => None,
                };
                if direct.is_some() {
                    // consume
                    self.s.next()
                } else {
                    Some(Tokens::Ident)
                }
            }
            t => {
                sign = false;
                direct = None;
                t
            }
        };

        match token {
            // {weekday} [{time}]
            // {month} [{day}, {year}] [{time}]
            // {month} [{day}] [{time}]
            Some(Tokens::Ident) => {
                let direct = direct.unwrap_or(Direction::Here);
                if let Some(month) = month_name(self.s.slice()) {
                    // {month} [{day}, {year}]
                    // {month} [{day}] [{time}]
                    if let Some(Tokens::Number(day)) = self.s.next() {
                        let s = self.s.clone();
                        if self.s.next() == Some(Tokens::Comma) {
                            // comma found, expect year
                            let year = self.next_num()?;
                            Ok(Some(DateSpec::absolute(year, month, day)))
                        } else {
                            // no comma found, we might expect a time component (if any)
                            // backtrack, we'll try parse the time component later
                            self.s = s;
                            Ok(Some(DateSpec::from_day_month(day, month, direct)))
                        }
                    } else {
                        // We only have a month name to work with
                        Ok(Some(DateSpec::FromName(ByName::MonthName(month), direct)))
                    }
                } else if let Some(weekday) = week_day(self.s.slice()) {
                    // {weekday} [{time}]
                    // we'll try parse the time component later
                    Ok(Some(DateSpec::FromName(ByName::WeekDay(weekday), direct)))
                } else {
                    Err(DateError::UnexpectedToken(
                        "week day or month name",
                        self.s.span(),
                    ))
                }
            }
            // {day}/{month}
            // {month}/{day}
            // {day} {month}
            // {n} {interval}
            // {year}-{month}-{day}
            Some(Tokens::Number(n)) => {
                match self.s.next() {
                    // if no extra tokens, this is probably just a year
                    None => Ok(Some(DateSpec::absolute(n, 1, 1))),
                    Some(Tokens::Ident) => {
                        let day = n;
                        let direct = direct.unwrap_or(Direction::Here);
                        let name = self.s.slice();
                        if let Some(month) = month_name(name) {
                            if let Some(Tokens::Number(year)) = self.s.next() {
                                // 4 July 2017
                                Ok(Some(DateSpec::absolute(year, month, day)))
                            } else {
                                // 4 July
                                Ok(Some(DateSpec::from_day_month(day, month, direct)))
                            }
                        } else if let Some(u) = time_unit(name) {
                            // '2 days'
                            if sign {
                                Ok(Some(DateSpec::skip(u, -(n as i32))))
                            } else {
                                match self.s.next() {
                                    Some(Tokens::Ident) => {
                                        if self.s.slice() == "ago" {
                                            Ok(Some(DateSpec::skip(u, -(n as i32))))
                                        } else {
                                            Err(DateError::UnexpectedToken("'ago'", self.s.span()))
                                        }
                                    }
                                    Some(Tokens::Number(h)) => {
                                        self.maybe_time = Some((h as u32, TimeKind::Unknown));

                                        Ok(Some(DateSpec::skip(u, n as i32)))
                                    }
                                    _ => Ok(Some(DateSpec::skip(u, n as i32))),
                                }
                            }
                        } else if name == "am" || name == "pm" {
                            self.maybe_time = Some((n, TimeKind::AmPm(name == "pm")));
                            Ok(None)
                        } else {
                            Err(DateError::UnexpectedToken(
                                "month or time unit",
                                self.s.span(),
                            ))
                        }
                    }
                    Some(Tokens::Colon) => {
                        self.maybe_time = Some((n, TimeKind::Formal));
                        Ok(None)
                    }
                    Some(Tokens::Dot) => {
                        self.maybe_time = Some((n, TimeKind::Informal));
                        Ok(None)
                    }
                    Some(Tokens::Dash) => Ok(Some(self.iso_date(n)?)),
                    Some(Tokens::Slash) => Ok(Some(self.informal_date(
                        n,
                        dialect,
                        direct.unwrap_or(Direction::Here),
                    )?)),
                    Some(_) => Err(DateError::UnexpectedToken("time", self.s.span())),
                }
            }
            Some(_) => Err(DateError::MissingDate),
            None => Err(DateError::UnexpectedEndOfText("empty date string")),
        }
    }

    fn formal_time(&mut self, hour: u32) -> DateResult<TimeSpec> {
        let min = self.next_num()?;
        // minute may be followed by [:secs][am|pm]
        let mut tnext = None;
        let sec = match self.s.next() {
            Some(Tokens::Colon) => self.next_num()?,
            Some(t @ (Tokens::Number(_) | Tokens::Ident)) => {
                tnext = Some(t);
                0
            }
            Some(_) => {
                return Err(DateError::UnexpectedToken("':'", self.s.span()));
            }
            None => 0,
        };
        // we found seconds, look ahead
        if tnext.is_none() {
            tnext = self.s.next();
        }
        let mut micros = 0;
        if let Some(Tokens::Dot) = tnext {
            // after a `.` implies these are subseconds.
            // We oly care for microsecond precision, so let's
            // get only the 6 most significant digits
            micros = self.next_num()?;
            while micros > 1_000_000 {
                micros /= 10;
            }
            tnext = self.s.next();
        };
        if let Some(tok) = tnext {
            match tok {
                Tokens::Plus | Tokens::Dash => {
                    let mut hours = self.next_num()?;
                    let minutes = if self.peek() == Tokens::Colon {
                        self.s.next(); // skip the colon

                        // 02:00
                        //    ^^
                        self.next_num()?
                    } else {
                        // 0030
                        //   ^^
                        let minutes = hours % 100;
                        hours /= 100;
                        minutes
                    };
                    let res = 60 * (minutes + 60 * hours);
                    let sign = if tok == Tokens::Dash { -1 } else { 1 };
                    let offset = (res as i64) * sign;
                    Ok(TimeSpec::new_with_offset(hour, min, sec, offset, micros))
                }
                Tokens::Ident => match self.s.slice() {
                    "Z" => Ok(TimeSpec::new_with_offset(hour, min, sec, 0, micros)),
                    "am" => Ok(TimeSpec::new(hour, min, sec, micros)),
                    "pm" => Ok(TimeSpec::new(hour + 12, min, sec, micros)),
                    _ => Err(DateError::UnexpectedToken(
                        "expected Z/am/pm",
                        self.s.span(),
                    )),
                },
                Tokens::Slash | Tokens::Colon | Tokens::Dot | Tokens::Comma | Tokens::Error => {
                    Err(DateError::UnexpectedToken("expected +/-", self.s.span()))
                }
                _ => Ok(TimeSpec::new(hour, min, sec, micros)),
            }
        } else {
            Ok(TimeSpec::new(hour, min, sec, micros))
        }
    }

    fn informal_time(&mut self, hour: u32) -> DateResult<TimeSpec> {
        let min = self.next_num()?;
        let hour = match self.s.next() {
            None => hour,
            Some(Tokens::Ident) if self.s.slice() == "am" => hour,
            Some(Tokens::Ident) if self.s.slice() == "pm" => hour + 12,
            Some(_) => return Err(DateError::UnexpectedToken("expected am/pm", self.s.span())),
        };
        Ok(TimeSpec::new(hour, min, 0, 0))
    }

    pub fn parse_time(&mut self) -> DateResult<Option<TimeSpec>> {
        // here the date parser looked ahead and saw an hour followed by some separator
        if let Some((h, kind)) = self.maybe_time {
            Ok(Some(match kind {
                TimeKind::Formal => self.formal_time(h)?,
                TimeKind::Informal => self.informal_time(h)?,
                TimeKind::AmPm(is_pm) => TimeSpec::new(if is_pm { h + 12 } else { h }, 0, 0, 0),
                TimeKind::Unknown => match self.s.next() {
                    Some(Tokens::Colon) => self.formal_time(h)?,
                    Some(Tokens::Dot) => self.informal_time(h)?,
                    Some(_) => return Err(DateError::UnexpectedToken(": or .", self.s.span())),
                    None => return Err(DateError::UnexpectedEndOfText(": or .")),
                },
            }))
        } else {
            // no lookahead...
            let mut peek = self.s.clone();
            if peek.next() == Some(Tokens::Ident) && peek.slice() == "T" {
                self.s.next();
            }
            let hour = match self.s.next() {
                None => return Ok(None),
                Some(Tokens::Number(n)) => n,
                Some(_) => return Err(DateError::UnexpectedToken("number", self.s.span())),
            };

            let time = match self.s.next() {
                Some(Tokens::Colon) => self.formal_time(hour)?,
                Some(Tokens::Dot) => self.informal_time(hour)?,
                Some(Tokens::Ident) => match self.s.slice() {
                    "am" => TimeSpec::new(hour, 0, 0, 0),
                    "pm" => TimeSpec::new(hour + 12, 0, 0, 0),
                    _ => return Err(DateError::UnexpectedToken("am/pm", self.s.span())),
                },
                Some(_) => {
                    return Err(DateError::UnexpectedToken(
                        "am/pm, ':' or '.'",
                        self.s.span(),
                    ))
                }
                None => return Err(DateError::UnexpectedEndOfText("am/pm, ':' or '.'")),
            };
            Ok(Some(time))
        }
    }

    pub fn parse(&mut self, dialect: Dialect) -> DateResult<DateTimeSpec> {
        let date = self.parse_date(dialect)?;
        let time = self.parse_time()?;
        Ok(DateTimeSpec { date, time })
    }
}
