use logos::{Lexer, Logos};

use crate::{
    types::{month_name, time_unit, ByName, DateSpec, DateTimeSpec, Direction, TimeSpec},
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
    pub(crate) dialect: Dialect,
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

impl Tokens {
    fn num(self) -> DateResult<u32> {
        match self {
            Tokens::Number(n) => Ok(n),
            t => Err(DateError::new(format!("expected number, found {t:?}"))),
        }
    }
    fn ok_or(self, msg: &str) -> DateResult<Self> {
        match self {
            Tokens::Error => Err(DateError::new(msg)),
            t => Ok(t),
        }
    }
}

impl<'a> DateParser<'a> {
    pub fn new(text: &'a str) -> DateParser<'a> {
        DateParser {
            s: Tokens::lexer(text),
            maybe_time: None,
            dialect: Dialect::Uk,
        }
    }

    fn next(&mut self) -> Tokens {
        self.s.next().unwrap_or(Tokens::Error)
    }
    fn next2(&mut self) -> Tokens {
        self.s.nth(1).unwrap_or(Tokens::Error)
    }
    fn peek(&self) -> Tokens {
        self.s.clone().next().unwrap_or(Tokens::Error)
    }

    pub fn dialect(mut self, d: Dialect) -> DateParser<'a> {
        self.dialect = d;
        self
    }

    fn iso_date(&mut self, y: u32) -> DateResult<DateSpec> {
        let month = self.next().num()?;
        if self.next() != Tokens::Dash {
            return Err(DateError::new("missing separator"));
        }
        let day = self.next().num()?;
        Ok(DateSpec::absolute(y, month, day))
    }

    fn informal_date(&mut self, day_or_month: u32, direct: Direction) -> DateResult<DateSpec> {
        let month_or_day = self.next().num()?;
        let (d, m) = if self.dialect == Dialect::Us {
            (month_or_day, day_or_month)
        } else {
            (day_or_month, month_or_day)
        };
        let date = if self.peek() == Tokens::Slash {
            let y = self.next2().num()?;
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

    fn parse_date(&mut self) -> DateResult<Option<DateSpec>> {
        let mut t = self.next().ok_or("empty date string")?;
        let sign = t == Tokens::Dash;
        if sign {
            t = self.next().ok_or("nothing after '-'")?
        }

        let mut direct = Direction::Here;
        match self.s.slice() {
            "now" | "today" => return Ok(Some(DateSpec::skip(Interval::Days(1), 0))),
            "yesterday" => return Ok(Some(DateSpec::skip(Interval::Days(1), -1))),
            "tomorrow" => return Ok(Some(DateSpec::skip(Interval::Days(1), 1))),
            "next" => direct = Direction::Next,
            "last" => direct = Direction::Last,
            _ => {}
        }

        if direct != Direction::Here {
            t = self.next().ok_or("nothing after last/next")?;
        }

        Ok(match t {
            Tokens::Ident => {
                let name = self.s.slice();
                // maybe weekday or month name?
                if let Some(by_name) = ByName::from_name(name) {
                    // however, MONTH _might_ be followed by DAY, YEAR
                    if let Some(month) = by_name.as_month() {
                        if let Some(Tokens::Number(day)) = self.s.next() {
                            let spec = if self.peek() == Tokens::Comma {
                                let year = self.next2().num()?;
                                DateSpec::absolute(year, month, day)
                            } else {
                                // MONTH DAY is like DAY MONTH (tho no time!)
                                DateSpec::from_day_month(day, month, direct)
                            };
                            return Ok(Some(spec));
                        }
                    }
                    Some(DateSpec::FromName(by_name, direct))
                } else {
                    return Err(DateError::new("expected week day or month name"));
                }
            }
            Tokens::Number(n) => {
                let t = match self.s.next() {
                    Some(t) => t,
                    None => return Ok(Some(DateSpec::absolute(n, 1, 1))),
                };
                match t {
                    Tokens::Ident => {
                        let day = n;
                        let name = self.s.slice();
                        if let Some(month) = month_name(name) {
                            if let Tokens::Number(year) = self.next() {
                                // 4 July 2017
                                Some(DateSpec::absolute(year, month, day))
                            } else {
                                // 4 July
                                Some(DateSpec::from_day_month(day, month, direct))
                            }
                        } else if let Some(u) = time_unit(name) {
                            // '2 days'
                            let mut n = n as i32;
                            if sign {
                                n = -n;
                            } else {
                                match self.next() {
                                    Tokens::Ident => {
                                        if self.s.slice() == "ago" {
                                            n = -n;
                                        } else {
                                            return Err(DateError::new("only expected 'ago'"));
                                        }
                                    }
                                    Tokens::Number(h) => {
                                        self.maybe_time = Some((h as u32, TimeKind::Unknown));
                                    }
                                    _ => {}
                                }
                            }
                            Some(DateSpec::skip(u, n))
                        } else if name == "am" || name == "pm" {
                            self.maybe_time = Some((n, TimeKind::AmPm(name == "pm")));
                            None
                        } else {
                            return Err(DateError::new("expected month or time unit"));
                        }
                    }
                    Tokens::Colon => {
                        self.maybe_time = Some((n, TimeKind::Formal));
                        None
                    }
                    Tokens::Dot => {
                        self.maybe_time = Some((n, TimeKind::Informal));
                        None
                    }
                    Tokens::Dash => Some(self.iso_date(n)?),
                    Tokens::Slash => Some(self.informal_date(n, direct)?),
                    _ => return Err(DateError::new(format!("unexpected token {t:?}"))),
                }
            }
            _ => return Err(DateError::new(format!("not expected token {t:?}"))),
        })
    }

    fn formal_time(&mut self, hour: u32) -> DateResult<TimeSpec> {
        let min = self.next().num()?;
        // minute may be followed by [:secs][am|pm]
        let mut tnext = None;
        let sec = match self.s.next() {
            Some(Tokens::Colon) => self.next().num()?,
            Some(t @ (Tokens::Number(_) | Tokens::Ident)) => {
                tnext = Some(t);
                0
            }
            Some(_) => {
                return Err(DateError::new("expecting ':'"));
            }
            None => 0,
        };
        // we found seconds, look ahead
        if tnext.is_none() {
            tnext = self.s.next();
        }
        let micros = if let Some(Tokens::Dot) = tnext {
            let frac = self.next().num()?;
            let micros_f = format!("0.{frac}").parse::<f64>().unwrap() * 1.0e6;
            tnext = self.s.next();
            micros_f as u32
        } else {
            0
        };
        if let Some(tok) = tnext {
            match tok {
                Tokens::Plus | Tokens::Dash => {
                    let h = self.next().num()?;
                    let (h, m) = if self.peek() == Tokens::Colon {
                        // 02:00
                        (h, self.next2().num()?)
                    } else {
                        // 0030 ....
                        let hh = h;
                        let h = hh / 100;
                        let m = hh % 100;
                        (h, m)
                    };
                    let res = 60 * (m + 60 * h);
                    let sign = if tok == Tokens::Dash { -1 } else { 1 };
                    let offset = (res as i64) * sign;
                    Ok(TimeSpec::new_with_offset(hour, min, sec, offset, micros))
                }
                Tokens::Ident => {
                    let s = self.s.slice();
                    if s == "Z" {
                        Ok(TimeSpec::new_with_offset(hour, min, sec, 0, micros))
                    } else {
                        // am or pm
                        let hour = DateParser::am_pm(s, hour)?;
                        Ok(TimeSpec::new(hour, min, sec, micros))
                    }
                }
                Tokens::Slash | Tokens::Colon | Tokens::Dot | Tokens::Comma | Tokens::Error => {
                    Err(DateError::new("expected +/- before timezone"))
                }
                _ => Ok(TimeSpec::new(hour, min, sec, micros)),
            }
        } else {
            Ok(TimeSpec::new(hour, min, sec, micros))
        }
    }

    fn informal_time(&mut self, hour: u32) -> DateResult<TimeSpec> {
        let min = self.next().num()?;
        let hour = match self.s.next() {
            Some(Tokens::Ident) => DateParser::am_pm(self.s.slice(), hour)?,
            Some(_) => return Err(DateError::new("expected am/pm")),
            None => hour,
        };
        Ok(TimeSpec::new(hour, min, 0, 0))
    }

    fn am_pm(name: &str, mut hour: u32) -> DateResult<u32> {
        if name == "pm" {
            hour += 12;
        } else if name != "am" {
            return Err(DateError::new("expected am or pm"));
        }
        Ok(hour)
    }

    fn hour_time(name: &str, hour: u32) -> DateResult<TimeSpec> {
        Ok(TimeSpec::new(DateParser::am_pm(name, hour)?, 0, 0, 0))
    }

    fn parse_time(&mut self) -> DateResult<Option<TimeSpec>> {
        // here the date parser looked ahead and saw an hour followed by some separator
        if let Some((h, kind)) = self.maybe_time {
            Ok(Some(match kind {
                TimeKind::Formal => self.formal_time(h)?,
                TimeKind::Informal => self.informal_time(h)?,
                TimeKind::AmPm(is_pm) => TimeSpec::new(if is_pm { h + 12 } else { h }, 0, 0, 0),
                TimeKind::Unknown => match self.next() {
                    Tokens::Colon => self.formal_time(h)?,
                    Tokens::Dot => self.informal_time(h)?,
                    _ => return Err(DateError::new(format!("expected : or ., not {}", self.s.slice()))),
                },
            }))
        } else {
            // no lookahead...
            let mut peek = self.s.clone();
            if peek.next() == Some(Tokens::Ident) && peek.slice() == "T" {
                self.next();
            }
            let hour = match self.s.next() {
                None => return Ok(None),
                Some(t) => t.num()?,
            };

            let time = match self.s.next() {
                Some(Tokens::Colon) => self.formal_time(hour)?,
                Some(Tokens::Dot) => self.informal_time(hour)?,
                Some(Tokens::Ident) => DateParser::hour_time(self.s.slice(), hour)?,
                Some(t) => return Err(DateError::new(format!("unexpected token {t:?}"))),
                None => return Err(DateError::new("unexpected eof")),
            };
            Ok(Some(time))
        }
    }

    pub fn parse(&mut self) -> DateResult<DateTimeSpec> {
        let date = self.parse_date()?;
        let time = self.parse_time()?;
        Ok(DateTimeSpec { date, time })
    }
}
