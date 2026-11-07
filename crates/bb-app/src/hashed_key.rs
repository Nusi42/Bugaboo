//! The place of a row in a list that is put in order by what the reader is shown, and cut in pages
//! by it (PRIV-09): what is shown (a moment, a date) and a hash of the ID of the row, which ties rows
//! of one moment or date and follows no order of making. It is all a cursor of such a list holds.
//! The history (`history::ShownKey`) and the entries of an issue (`time::DayKey`) both use it.

use std::fmt::{self, Display};
use std::str::FromStr;

/// What a row is shown at, and the hash of its ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HashedKey<T> {
    /// What the row is shown at: a moment in microseconds, or a date.
    pub at: T,
    /// The hash of the ID of the row (`history::opaque_id`), as hexadecimal text.
    pub hash: String,
}

impl<T: Display> Display for HashedKey<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.at, self.hash)
    }
}

impl<T: FromStr> FromStr for HashedKey<T> {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        let (at, hash) = text.split_once(':').ok_or(())?;
        let clean = !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit());
        clean
            .then(|| at.parse().ok())
            .flatten()
            .map(|at| Self {
                at,
                hash: hash.to_owned(),
            })
            .ok_or(())
    }
}

/// What a fake store does for a keyset page: the rows after the key, newest first by key, with `skip`
/// rows passed over and at most `limit` taken.
#[cfg(test)]
pub(crate) fn fake_page<E, T: Ord + Copy>(
    mut rows: Vec<(E, HashedKey<T>)>,
    after: Option<&HashedKey<T>>,
    skip: u64,
    limit: u64,
) -> Vec<(E, HashedKey<T>)> {
    rows.retain(|(_, key)| after.is_none_or(|after| (key.at, &key.hash) < (after.at, &after.hash)));
    rows.sort_by(|(_, one), (_, other)| (other.at, &other.hash).cmp(&(one.at, &one.hash)));
    rows.into_iter()
        .skip(usize::try_from(skip).unwrap_or(usize::MAX))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect()
}

#[cfg(test)]
mod tests {
    use jiff::civil::Date;

    use super::*;

    #[test]
    fn priv_09_a_key_reads_back_and_refuses_what_it_did_not_write() {
        let moment = HashedKey {
            at: 1_800_000_000_000_000_i64,
            hash: "0aff".to_owned(),
        };
        assert_eq!(moment.to_string().parse(), Ok(moment));
        let day = HashedKey {
            at: Date::new(2027, 1, 14).unwrap_or_default(),
            hash: "00ff".to_owned(),
        };
        assert_eq!(day.to_string().parse(), Ok(day));
        for text in ["", "x", "1:", "1:zz", "a:0aff", ":0aff"] {
            assert!(text.parse::<HashedKey<i64>>().is_err(), "{text}");
        }
        assert!("2027-13-40:0aff".parse::<HashedKey<Date>>().is_err());
    }
}
