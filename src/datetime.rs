mod sealed {
    pub trait Sealed {}
}

pub trait Date: Clone + PartialOrd + sealed::Sealed {
    #[doc(hidden)]
    fn from_ymd(year: i32, month: u8, day: u8) -> Option<Self>;
    #[doc(hidden)]
    fn offset_months(self, months: i32) -> Option<Self>;
    #[doc(hidden)]
    fn offset_days(self, days: i64) -> Option<Self>;

    #[doc(hidden)]
    fn year(&self) -> i32;
    #[doc(hidden)]
    fn weekday(&self) -> u8;
}

pub trait Time: Clone + PartialOrd + sealed::Sealed{
    #[doc(hidden)]
    fn from_hms(h: u32, m: u32, s: u32) -> Option<Self>;
    #[doc(hidden)]
    fn with_micros(self, ms: u32) -> Option<Self>;
}

pub trait DateTime: Sized + sealed::Sealed {
    type TimeZone;
    type Date: Date;
    type Time: Time;

    #[doc(hidden)]
    fn new(tz: Self::TimeZone, date: Self::Date, time: Self::Time) -> Self;
    #[doc(hidden)]
    fn split(self) -> (Self::TimeZone, Self::Date, Self::Time);
    #[doc(hidden)]
    fn with_offset(self, secs: i64) -> Option<Self>;
    #[doc(hidden)]
    fn offset_seconds(self, secs: i64) -> Option<Self>;
}

#[cfg(feature = "chrono")]
mod chrono {
    use chrono::{Duration, NaiveDate, NaiveTime, Offset, TimeZone, Timelike};

    impl super::sealed::Sealed for NaiveDate {}
    impl super::sealed::Sealed for NaiveTime {}
    impl<Tz: TimeZone> super::sealed::Sealed for chrono::DateTime<Tz> {}

    use super::{Date, DateTime, Time};
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
    impl<Tz: TimeZone> DateTime for chrono::DateTime<Tz> {
        type TimeZone = Tz::Offset;
        type Date = NaiveDate;
        type Time = NaiveTime;

        fn new(tz: Self::TimeZone, date: Self::Date, time: Self::Time) -> Self {
            Self::from_naive_utc_and_offset(date.and_time(time) - tz.fix(), tz)
        }

        fn split(self) -> (Self::TimeZone, Self::Date, Self::Time) {
            (self.offset().clone(), self.date_naive(), self.time())
        }

        fn with_offset(self, secs: i64) -> Option<Self> {
            let offset = self
                .timezone()
                .offset_from_utc_date(&self.date_naive())
                .fix()
                .local_minus_utc() as i64;
            self.offset_seconds(offset - secs)
        }

        fn offset_seconds(self, secs: i64) -> Option<Self> {
            self.checked_add_signed(Duration::seconds(secs))
        }
    }
}

#[cfg(feature = "time")]
mod time {
    use super::{Date, DateTime, Time};

    impl super::sealed::Sealed for time::Date {}
    impl super::sealed::Sealed for time::Time {}
    impl super::sealed::Sealed for time::OffsetDateTime {}

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
            m = time::Month::try_from(m1).ok()?;

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

        fn with_offset(self, secs: i64) -> Option<Self> {
            let offset = self.offset().whole_seconds() as i64;
            self.offset_seconds(offset - secs)
        }

        fn offset_seconds(self, secs: i64) -> Option<Self> {
            self.checked_add(time::Duration::seconds(secs))
        }
    }
}
