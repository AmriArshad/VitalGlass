//! Unit and wire-format conversions, kept in one place.
//!
//! The API sends distances and elevations in millimetres, 64-bit integers
//! as JSON strings (`"12183"`), and durations/UTC offsets as protobuf
//! duration strings (`"3600s"`, `"-43200s"`, `"1.5s"`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer};

/// Millimetres to metres. The store keeps metres; the UI picks km or miles.
pub fn mm_to_m(mm: f64) -> f64 {
    mm / 1000.0
}

/// Parse a protobuf duration string like `"46800s"` into whole seconds.
pub fn duration_secs(s: &str) -> Option<i64> {
    let n: f64 = s.strip_suffix('s')?.parse().ok()?;
    Some(n.round() as i64)
}

/// The local calendar date of `t`, given its UTC offset string. The API
/// often omits the `civil*Time` fields, so derive the date from the offset
/// rather than using the UTC date (which is a day behind for NZ mornings).
pub fn civil_date(t: DateTime<Utc>, utc_offset: Option<&str>) -> NaiveDate {
    let offset = utc_offset.and_then(duration_secs).unwrap_or(0);
    (t + chrono::Duration::seconds(offset)).date_naive()
}

/// Deserialize an int64 sent either as a JSON string or a number.
pub fn de_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrNum {
        Str(String),
        Num(i64),
    }
    match StrOrNum::deserialize(d)? {
        StrOrNum::Num(n) => Ok(n),
        StrOrNum::Str(s) => s.parse().map_err(serde::de::Error::custom),
    }
}

/// [`de_i64`] for optional fields. Use with `#[serde(default)]`.
pub fn de_opt_i64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    #[derive(Deserialize)]
    struct Wrap(#[serde(deserialize_with = "de_i64")] i64);
    Ok(Option::<Wrap>::deserialize(d)?.map(|w| w.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(duration_secs("46800s"), Some(46800));
        assert_eq!(duration_secs("-43200s"), Some(-43200));
        assert_eq!(duration_secs("1.6s"), Some(2));
        assert_eq!(duration_secs("12"), None);
    }

    #[test]
    fn civil_dates() {
        // 08:57 NZDT on the 24th is 19:57 UTC on the 23rd.
        let t: DateTime<Utc> = "2026-10-23T19:57:00Z".parse().unwrap();
        assert_eq!(civil_date(t, Some("46800s")).to_string(), "2026-10-24");
        assert_eq!(civil_date(t, None).to_string(), "2026-10-23");
    }

    #[test]
    fn int64_strings() {
        #[derive(Deserialize)]
        struct T {
            #[serde(deserialize_with = "de_i64")]
            a: i64,
            #[serde(default, deserialize_with = "de_opt_i64")]
            b: Option<i64>,
            #[serde(default, deserialize_with = "de_opt_i64")]
            c: Option<i64>,
        }
        let t: T = serde_json::from_str(r#"{"a": "12183", "b": 7}"#).unwrap();
        assert_eq!((t.a, t.b, t.c), (12183, Some(7), None));
    }
}
