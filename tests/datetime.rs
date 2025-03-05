use std::fmt::Debug;

use interim::{datetime::DateTime, parse_date_string, Dialect};

#[allow(unused)]
trait FormatDateTime: DateTime + Debug {
    /// Datetime representing 2018-03-21T11:00:00+02:00
    fn base() -> Self;
    /// RFC3339 format, eg 2018-03-21T11:00:00+02:00
    fn format(&self) -> String;
}

#[cfg(feature = "chrono_0_4")]
mod chrono_0_4 {
    use super::*;

    use chrono::{DateTime, FixedOffset, TimeZone};

    impl FormatDateTime for DateTime<FixedOffset> {
        fn base() -> Self {
            FixedOffset::east_opt(7200)
                .unwrap()
                .with_ymd_and_hms(2018, 3, 21, 11, 00, 00)
                .unwrap()
        }
        fn format(&self) -> String {
            self.format("%+").to_string()
        }
    }

    #[test]
    fn acceptance() {
        super::acceptance::<DateTime<FixedOffset>>();
    }

    #[test]
    /// <https://github.com/conradludgate/interim/issues/12>
    fn regression_12() {
        use chrono::TimeZone;

        let now: chrono::DateTime<_> = chrono_tz::America::Los_Angeles
            .with_ymd_and_hms(2024, 1, 1, 12, 00, 00)
            .unwrap();
        let without_timezone = parse_date_string("2024-06-01 12:00:00", now, Dialect::Us).unwrap();
        let with_timezone =
            parse_date_string("2024-06-01 12:00:00 -07:00", now, Dialect::Us).unwrap();

        assert_eq!(without_timezone, with_timezone);
        assert_eq!(with_timezone.to_string(), "2024-06-01 12:00:00 PDT");
    }
}

#[cfg(feature = "time_0_3")]
mod time_0_3 {
    use super::*;

    use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

    impl FormatDateTime for OffsetDateTime {
        fn base() -> Self {
            PrimitiveDateTime::new(
                Date::from_calendar_date(2018, Month::March, 21).unwrap(),
                Time::from_hms(11, 00, 00).unwrap(),
            )
            .assume_offset(UtcOffset::from_whole_seconds(7200).unwrap())
        }
        fn format(&self) -> String {
            let format = time::format_description::parse(
                "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]",
            ).unwrap();

            OffsetDateTime::format(*self, &format).unwrap()
        }
    }

    #[test]
    fn acceptance() {
        super::acceptance::<OffsetDateTime>();
    }
}

#[cfg(feature = "jiff_0_1")]
mod jiff_0_1 {
    use super::*;

    use ::jiff_0_1::{civil::Date, civil::DateTime, civil::Time, tz::Offset, tz::TimeZone, Zoned};

    impl FormatDateTime for Zoned {
        fn base() -> Self {
            let tz = TimeZone::fixed(Offset::from_seconds(7200).unwrap());
            let base =
                DateTime::from_parts(Date::constant(2018, 3, 21), Time::constant(11, 00, 00, 0));
            tz.to_zoned(base).unwrap()
        }
        fn format(&self) -> String {
            self.strftime("%FT%T%:z").to_string()
        }
    }

    #[test]
    fn acceptance() {
        super::acceptance::<Zoned>();
    }

    #[test]
    fn regression_12() {
        let tz = TimeZone::get("America/Los_Angeles").unwrap();
        let base = DateTime::from_parts(Date::constant(2024, 1, 1), Time::constant(12, 00, 00, 0));
        let now = tz.to_zoned(base).unwrap();

        let without_timezone =
            parse_date_string("2024-06-01 12:00:00", now.clone(), Dialect::Us).unwrap();
        let with_timezone =
            parse_date_string("2024-06-01 12:00:00 -07:00", now, Dialect::Us).unwrap();

        assert_eq!(without_timezone, with_timezone);
        assert_eq!(
            with_timezone.to_string(),
            "2024-06-01T12:00:00-07:00[America/Los_Angeles]"
        );
    }
}

#[cfg(feature = "jiff_0_2")]
mod jiff_0_2 {
    use super::*;

    use ::jiff_0_2::{civil::Date, civil::DateTime, civil::Time, tz::Offset, tz::TimeZone, Zoned};

    impl FormatDateTime for Zoned {
        fn base() -> Self {
            let tz = TimeZone::fixed(Offset::from_seconds(7200).unwrap());
            let base =
                DateTime::from_parts(Date::constant(2018, 3, 21), Time::constant(11, 00, 00, 0));
            tz.to_zoned(base).unwrap()
        }
        fn format(&self) -> String {
            self.strftime("%FT%T%:z").to_string()
        }
    }

    #[test]
    fn acceptance() {
        super::acceptance::<Zoned>();
    }

    #[test]
    fn regression_12() {
        let tz = TimeZone::get("America/Los_Angeles").unwrap();
        let base = DateTime::from_parts(Date::constant(2024, 1, 1), Time::constant(12, 00, 00, 0));
        let now = tz.to_zoned(base).unwrap();

        let without_timezone =
            parse_date_string("2024-06-01 12:00:00", now.clone(), Dialect::Us).unwrap();
        let with_timezone =
            parse_date_string("2024-06-01 12:00:00 -07:00", now, Dialect::Us).unwrap();

        assert_eq!(without_timezone, with_timezone);
        assert_eq!(
            with_timezone.to_string(),
            "2024-06-01T12:00:00-07:00[America/Los_Angeles]"
        );
    }
}

#[allow(unused)]
fn acceptance<Dt: FormatDateTime>() {
    use Dialect::{Uk, Us};
    fn assert<Dt: FormatDateTime>(input: &str, dialect: Dialect, expected: &str) {
        let date = match parse_date_string(input, Dt::base(), dialect) {
            Ok(date) => date,
            Err(e) => {
                panic!("unexpected error attempting to parse {input:?}\n\t{e:?}")
            }
        };

        let actual = date.format();
        assert_eq!(actual, expected, "unexpected output attempting to format {input:?}.\nexpected: {expected:?}\n  parsed: {date:?} [{actual:?}]");
    }

    // Day of week - relative to today. May have a time part
    assert::<Dt>("friday", Uk, "2018-03-23T00:00:00+02:00");
    assert::<Dt>("friday 10:30", Uk, "2018-03-23T10:30:00+02:00");
    assert::<Dt>("friday 8pm", Uk, "2018-03-23T20:00:00+02:00");
    assert::<Dt>("12am", Uk, "2018-03-21T00:00:00+02:00");
    assert::<Dt>("12pm", Uk, "2018-03-21T12:00:00+02:00");
    assert::<Dt>("7:26 AM", Uk, "2018-03-21T07:26:00+02:00");
    assert::<Dt>("7:26 PM", Uk, "2018-03-21T19:26:00+02:00");

    // The day of week is the _next_ day after today, so "Tuesday" is the next Tuesday after Wednesday
    assert::<Dt>("tues", Uk, "2018-03-27T00:00:00+02:00");

    // The expression 'next Monday' is ambiguous; in the US it means the day following (same as 'Monday')
    // (This is how the `date` command interprets it)
    assert::<Dt>("next mon", Us, "2018-03-26T00:00:00+02:00");
    // but otherwise it means the day in the next week..
    assert::<Dt>("next mon", Uk, "2018-04-02T00:00:00+02:00");

    assert::<Dt>("last year", Uk, "2017-03-21T00:00:00+02:00");
    assert::<Dt>("this year", Uk, "2018-03-21T00:00:00+02:00");
    assert::<Dt>("next year", Uk, "2019-03-21T00:00:00+02:00");

    assert::<Dt>("last fri 9.30", Uk, "2018-03-16T09:30:00+02:00");

    // date expressed as month, day - relative to today. May have a time part
    assert::<Dt>("8/11", Us, "2018-08-11T00:00:00+02:00");
    assert::<Dt>("last 8/11", Us, "2017-08-11T00:00:00+02:00");
    assert::<Dt>("last 8/11 9am", Us, "2017-08-11T09:00:00+02:00");
    assert::<Dt>("8/11", Uk, "2018-11-08T00:00:00+02:00");
    assert::<Dt>("last 8/11", Uk, "2017-11-08T00:00:00+02:00");
    assert::<Dt>("last 8/11 9am", Uk, "2017-11-08T09:00:00+02:00");
    assert::<Dt>("April 1 8.30pm", Uk, "2018-04-01T20:30:00+02:00");

    // advance by time unit from today
    // without explicit time, use base time - otherwise override
    assert::<Dt>("2d", Uk, "2018-03-23T11:00:00+02:00");
    assert::<Dt>("2d 03:00", Uk, "2018-03-23T03:00:00+02:00");
    assert::<Dt>("3 weeks", Uk, "2018-04-11T11:00:00+02:00");
    assert::<Dt>("3h", Uk, "2018-03-21T14:00:00+02:00");
    assert::<Dt>("6 months", Uk, "2018-09-21T00:00:00+02:00");
    assert::<Dt>("6 months ago", Uk, "2017-09-21T00:00:00+02:00");
    assert::<Dt>("3 hours ago", Uk, "2018-03-21T08:00:00+02:00");
    assert::<Dt>(" -3h", Uk, "2018-03-21T08:00:00+02:00");
    assert::<Dt>(" -3 month", Uk, "2017-12-21T00:00:00+02:00");

    // absolute date with year, month, day - formal ISO and informal UK or US
    assert::<Dt>("2017-06-30", Uk, "2017-06-30T00:00:00+02:00");
    assert::<Dt>("30/06/17", Uk, "2017-06-30T00:00:00+02:00");
    assert::<Dt>("06/30/17", Us, "2017-06-30T00:00:00+02:00");

    // may be followed by time part, formal and informal
    assert::<Dt>("2017-06-30 08:20:30", Uk, "2017-06-30T08:20:30+02:00");
    assert::<Dt>(
        "2017-06-30 08:20:30 +04:00",
        Uk,
        "2017-06-30T06:20:30+02:00",
    );
    assert::<Dt>("2017-06-30 08:20:30 +0400", Uk, "2017-06-30T06:20:30+02:00");
    assert::<Dt>("2017-06-30T08:20:30Z", Uk, "2017-06-30T10:20:30+02:00");
    assert::<Dt>("2017-06-30T08:20:30", Uk, "2017-06-30T08:20:30+02:00");
    assert::<Dt>("2017-06-30 12.20", Uk, "2017-06-30T12:20:00+02:00");
    assert::<Dt>("2017-06-30 8.20", Uk, "2017-06-30T08:20:00+02:00");
    assert::<Dt>("2017-06-30 12.15am", Uk, "2017-06-30T00:15:00+02:00");
    assert::<Dt>("2017-06-30 12.25pm", Uk, "2017-06-30T12:25:00+02:00");
    assert::<Dt>("2017-06-30 12:30pm", Uk, "2017-06-30T12:30:00+02:00");
    assert::<Dt>("2017-06-30 8.30pm", Uk, "2017-06-30T20:30:00+02:00");
    assert::<Dt>("2017-06-30 8:30pm", Uk, "2017-06-30T20:30:00+02:00");
    assert::<Dt>("2017-06-30 2am", Uk, "2017-06-30T02:00:00+02:00");
    assert::<Dt>("30 June 2018", Uk, "2018-06-30T00:00:00+02:00");
    assert::<Dt>("June 30, 2018", Uk, "2018-06-30T00:00:00+02:00");
    assert::<Dt>("June   30,    2018", Uk, "2018-06-30T00:00:00+02:00");
}
