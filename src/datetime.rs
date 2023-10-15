pub trait Date: Clone + PartialOrd {
    fn from_ymd(year: i32, month: u8, day: u8) -> Option<Self>;
    fn offset_months(self, months: i32) -> Option<Self>;
    fn offset_days(self, days: i64) -> Option<Self>;

    fn year(&self) -> i32;
    fn weekday(&self) -> u8;
}

pub trait Time: Clone + PartialOrd {
    fn from_hms(h: u32, m: u32, s: u32) -> Option<Self>;
    fn with_micros(self, ms: u32) -> Option<Self>;
}

pub trait DateTime: Sized {
    type TimeZone: Timezone;
    type Date: Date;
    type Time: Time;

    fn new(tz: Self::TimeZone, date: Self::Date, time: Self::Time) -> Self;
    fn split(self) -> (Self::TimeZone, Self::Date, Self::Time);
    fn offset_seconds(self, secs: i64) -> Option<Self>;
}

pub trait Timezone {
    fn local_minus_utc(&self) -> i64;
}

#[cfg(feature = "chrono")]
mod chrono {
    use chrono::{Duration, NaiveDate, NaiveTime, TimeZone, Timelike};

    use super::{Date, DateTime, Time, Timezone};
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    impl Date for NaiveDate {
        fn from_ymd(year: i32, month: u8, day: u8) -> Option<Self> {
            NaiveDate::from_ymd_opt(year, month as u32, day as u32)
        }

        fn offset_months(self, months: i32) -> Option<Self> {
            if months >= 0 {
                self.checked_add_months(chrono::Months::new(months as u32))
            } else {
                self.checked_sub_months(chrono::Months::new(-months as u32))
            }
        }

        fn offset_days(self, days: i64) -> Option<Self> {
            self.checked_add_signed(Duration::days(days))
        }

        fn year(&self) -> i32 {
            chrono::Datelike::year(self)
        }

        fn weekday(&self) -> u8 {
            chrono::Datelike::weekday(self).num_days_from_monday() as u8
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    impl Time for NaiveTime {
        fn from_hms(h: u32, m: u32, s: u32) -> Option<Self> {
            NaiveTime::from_hms_opt(h, m, s)
        }
        fn with_micros(self, ms: u32) -> Option<Self> {
            self.with_nanosecond(ms.checked_mul(1_000)?)
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    impl<Tz: TimeZone> DateTime for chrono::DateTime<Tz>
    where
        Tz::Offset: Timezone,
    {
        type TimeZone = Tz::Offset;
        type Date = NaiveDate;
        type Time = NaiveTime;

        fn new(tz: Self::TimeZone, date: Self::Date, time: Self::Time) -> Self {
            chrono::DateTime::from_local(date.and_time(time), tz)
        }

        fn split(self) -> (Self::TimeZone, Self::Date, Self::Time) {
            (self.offset().clone(), self.date_naive(), self.time())
        }

        fn offset_seconds(self, secs: i64) -> Option<Self> {
            self.checked_add_signed(Duration::seconds(secs))
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    impl Timezone for chrono::FixedOffset {
        fn local_minus_utc(&self) -> i64 {
            self.local_minus_utc() as i64
        }
    }
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    impl Timezone for chrono::Utc {
        fn local_minus_utc(&self) -> i64 {
            0
        }
    }
}

#[cfg(feature = "time")]
mod time {
    use super::{Date, DateTime, Time, Timezone};

    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    impl Date for time::Date {
        fn from_ymd(year: i32, month: u8, day: u8) -> Option<Self> {
            time::Date::from_calendar_date(year, time::Month::try_from(month).ok()?, day).ok()
        }

        fn offset_months(self, months: i32) -> Option<Self> {
            // need to calculate this manually :(
            let (mut y, mut m, d) = self.to_calendar_date();

            y += months / 12;
            let mut months = months % 12;
            if months < 0 {
                months += 12;
                y -= 1;
            }

            // months will be between 0..12
            let mut m1 = m as u8 + months as u8;
            if m1 > 12 {
                m1 -= 12;
                y += 1;
            }
            m = time::Month::try_from(m1 as u8).ok()?;

            let max_day = time::util::days_in_year_month(y, m);
            let d = d.min(max_day);
            time::Date::from_calendar_date(y, m, d).ok()
        }

        fn offset_days(self, days: i64) -> Option<Self> {
            self.checked_add(time::Duration::days(days))
        }

        fn year(&self) -> i32 {
            time::Date::year(*self)
        }
        fn weekday(&self) -> u8 {
            time::Date::weekday(*self).number_days_from_monday()
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    impl Time for time::Time {
        fn from_hms(h: u32, m: u32, s: u32) -> Option<Self> {
            time::Time::from_hms(
                u8::try_from(h).ok()?,
                u8::try_from(m).ok()?,
                u8::try_from(s).ok()?,
            )
            .ok()
        }

        fn with_micros(self, ms: u32) -> Option<Self> {
            self.replace_microsecond(ms).ok()
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    impl DateTime for time::OffsetDateTime {
        type TimeZone = time::UtcOffset;
        type Date = time::Date;
        type Time = time::Time;

        fn new(tz: Self::TimeZone, date: Self::Date, time: Self::Time) -> Self {
            time::PrimitiveDateTime::new(date, time).assume_offset(tz)
        }

        fn split(self) -> (Self::TimeZone, Self::Date, Self::Time) {
            (self.offset(), self.date(), self.time())
        }

        fn offset_seconds(self, secs: i64) -> Option<Self> {
            self.checked_add(time::Duration::seconds(secs))
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    impl Timezone for time::UtcOffset {
        fn local_minus_utc(&self) -> i64 {
            self.whole_seconds() as i64
        }
    }
}
