pub fn main() {
    #[allow(unused_variables)]
    let arg = std::env::args().skip(1).collect::<Vec<String>>().join(" ");

    #[cfg(feature = "time")]
    {
        use interim::{parse_date_string, Dialect};
        use time::OffsetDateTime;
        println!(
            "{}",
            parse_date_string(arg.as_str(), OffsetDateTime::now_utc(), Dialect::Us).unwrap()
        );
    }
    #[cfg(feature = "chrono")]
    {
        use chrono::Local;
        use interim::{parse_date_string, Dialect};
        println!(
            "{}",
            parse_date_string(arg.as_str(), Local::now(), Dialect::Us).unwrap()
        );
    }
    #[cfg(not(any(feature = "time", feature = "chrono")))]
    {
        eprintln!("Please enable either time or chrono feature")
    }
}
