//! Civil dates can be valid while their zoned timestamps are out of range.

#[cfg(any(feature = "jiff_0_1", feature = "jiff_0_2"))]
macro_rules! range_tests {
    ($provider:ident) => {
        use $provider::{
            civil::{Date, DateTime, Time},
            tz::{Offset, TimeZone},
            Zoned,
        };
        use interim::{parse_date_string, DateError, Dialect};

        fn base(hours: i32, year: i16, month: i8, day: i8) -> Zoned {
            TimeZone::fixed(Offset::from_seconds(hours * 3600).unwrap())
                .to_zoned(DateTime::from_parts(
                    Date::new(year, month, day).unwrap(),
                    Time::constant(0, 0, 0, 0),
                ))
                .unwrap()
        }

        #[test]
        fn absolute_date_range() {
            for hours in [-12, 0, 14] {
                for input in ["9999-12-31", "9999-12-31 00:00:00"] {
                    let result = parse_date_string(input, base(hours, 2018, 3, 21), Dialect::Uk);
                    if hours == 14 {
                        assert_eq!(result.unwrap().date(), Date::constant(9999, 12, 31));
                    } else {
                        assert_eq!(result.unwrap_err(), DateError::MissingDate);
                    }
                }
                assert_eq!(
                    parse_date_string(
                        "9999-12-31 23:59:59",
                        base(hours, 2018, 3, 21),
                        Dialect::Uk,
                    )
                    .unwrap_err(),
                    DateError::MissingDate,
                );
            }
        }

        #[test]
        fn relative_and_named_date_range() {
            for hours in [-12, 0, 14] {
                for (input, start) in [
                    ("2 days", base(hours, 9999, 12, 29)),
                    ("2 months", base(hours, 9999, 10, 31)),
                    ("31 December", base(hours, 9999, 10, 31)),
                ] {
                    let result = parse_date_string(input, start, Dialect::Uk);
                    if hours == 14 {
                        assert_eq!(result.unwrap().date(), Date::constant(9999, 12, 31));
                    } else {
                        assert_eq!(result.unwrap_err(), DateError::MissingDate);
                    }
                }
                assert_eq!(
                    parse_date_string(
                        "2 months 23:59:59",
                        base(hours, 9999, 10, 31),
                        Dialect::Uk,
                    )
                    .unwrap_err(),
                    DateError::MissingDate,
                );
            }
        }

        #[test]
        fn time_only_range() {
            for hours in [-12, 0, 14] {
                let result = parse_date_string(
                    "23:59:59",
                    base(hours, 9999, 12, 30),
                    Dialect::Uk,
                );
                if hours == 14 {
                    assert_eq!(result.unwrap().time(), Time::constant(23, 59, 59, 0));
                } else {
                    assert_eq!(result.unwrap_err(), DateError::MissingTime);
                }
            }
        }

        #[test]
        fn representable_dates_still_work() {
            for hours in [-12, 0, 14] {
                for (input, start, expected) in [
                    (
                        "1 day",
                        base(hours, 9999, 12, 29),
                        Date::constant(9999, 12, 30),
                    ),
                    (
                        "1 month",
                        base(hours, 9999, 10, 31),
                        Date::constant(9999, 11, 30),
                    ),
                    (
                        "0000-01-01",
                        base(hours, 2018, 3, 21),
                        Date::constant(0, 1, 1),
                    ),
                    (
                        "2018-03-22",
                        base(hours, 2018, 3, 21),
                        Date::constant(2018, 3, 22),
                    ),
                ] {
                    assert_eq!(
                        parse_date_string(input, start, Dialect::Uk).unwrap().date(),
                        expected,
                    );
                }
                let zoned = parse_date_string(
                    "2018-03-22 12:00:00 +02:00",
                    base(hours, 2018, 3, 21),
                    Dialect::Uk,
                )
                .unwrap();
                assert_eq!(zoned.timestamp().as_second(), 1_521_712_800);
            }
        }
    };
}

#[cfg(feature = "jiff_0_1")]
mod jiff_0_1 {
    range_tests!(jiff_0_1);
}

#[cfg(feature = "jiff_0_2")]
mod jiff_0_2 {
    range_tests!(jiff_0_2);
}
