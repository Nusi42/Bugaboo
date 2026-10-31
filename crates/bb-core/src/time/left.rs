//! *Time left* of an issue, worked out once (ADR-0035, decision 4): the value somebody typed while
//! there is one, else *Estimated time* minus *Time spent*, never below zero. The issue answer, the
//! list and the totals ask this function; the search compiler writes the same rule as one SQL
//! expression, held to this one by the vectors of [`time_left_file`]
//! (`tools/testdata/time-left.json`, coding-standards §1, the second approved pair).

use serde::Serialize;
use serde_json::{Value, json};

use super::Minutes;

/// *Time left* as it is shown: typed by somebody, or worked out from the estimate and the time
/// spent (the screens show the second in the quiet style of a value worked out).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "minutes", rename_all = "kebab-case")]
pub enum TimeLeft {
    /// Somebody typed it; it counts until it is cleared.
    Typed(Minutes),
    /// Nobody typed it: the estimate minus the time spent, never below zero.
    WorkedOut(Minutes),
}

impl TimeLeft {
    /// The minutes, typed or worked out.
    #[must_use]
    pub const fn minutes(self) -> Minutes {
        match self {
            Self::Typed(minutes) | Self::WorkedOut(minutes) => minutes,
        }
    }
}

/// *Time left* of an issue with this estimate, this time spent and the value somebody typed:
/// the typed value while there is one, else the estimate minus the time spent and never below
/// zero; none for an issue with neither an estimate nor a typed value.
#[must_use]
pub fn time_left(
    estimated: Option<Minutes>,
    spent: Minutes,
    typed: Option<Minutes>,
) -> Option<TimeLeft> {
    match (typed, estimated) {
        (Some(typed), _) => Some(TimeLeft::Typed(typed)),
        (None, Some(estimated)) => Some(TimeLeft::WorkedOut(
            estimated.checked_sub(spent).unwrap_or_default(),
        )),
        (None, None) => None,
    }
}

/// The cases of the vectors: the estimate, the time spent and the typed value, in minutes.
const CASES: [(Option<u32>, u32, Option<u32>); 12] = [
    (None, 0, None),
    (None, 90, None),
    (Some(240), 0, None),
    (Some(240), 80, None),
    (Some(240), 240, None),
    (Some(240), 300, None),
    (Some(0), 30, None),
    (Some(240), 80, Some(30)),
    (Some(240), 300, Some(0)),
    (None, 90, Some(45)),
    (None, 0, Some(0)),
    (Some(4_294_967_295), 1, None),
];

/// The file of the vectors (`tools/testdata/time-left.json`), as `cargo xtask gen time-vectors`
/// writes it: every case with the answer of [`time_left`], in minutes, `null` for none.
///
/// # Errors
/// When the JSON cannot be written.
pub fn time_left_file() -> Result<String, serde_json::Error> {
    let cases: Vec<Value> = CASES
        .iter()
        .map(|(estimated, spent, typed)| {
            let answer = time_left(
                estimated.map(Minutes::new),
                Minutes::new(*spent),
                typed.map(Minutes::new),
            );
            json!({
                "estimated": estimated,
                "spent": spent,
                "typed": typed,
                "left": answer.map(|left| left.minutes().get()),
            })
        })
        .collect();
    let file = json!({
        "about": "The worked-out Time left of an issue: the typed value while there is one, else the estimate minus the time spent, never below zero, and none without either (ADR-0035, decision 4). This file is written by `cargo xtask gen time-vectors` from crates/bb-core/src/time/left.rs; the two-database test `time_03_the_compiled_time_left_matches_the_vectors` runs every case through the expression of the search compiler. All values are minutes; null is none.",
        "cases": cases,
    });
    let mut text = serde_json::to_string_pretty(&file)?;
    text.push('\n');
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn minutes(count: u32) -> Minutes {
        Minutes::new(count)
    }

    #[test]
    fn time_03_time_left_is_the_estimate_minus_the_time_spent_until_someone_types_it() {
        // Worked out while nobody typed it.
        assert_eq!(
            time_left(Some(minutes(240)), minutes(80), None),
            Some(TimeLeft::WorkedOut(minutes(160)))
        );
        // Never below zero.
        assert_eq!(
            time_left(Some(minutes(240)), minutes(300), None),
            Some(TimeLeft::WorkedOut(minutes(0)))
        );
        // A typed value wins, also a typed zero, also without an estimate.
        assert_eq!(
            time_left(Some(minutes(240)), minutes(80), Some(minutes(30))),
            Some(TimeLeft::Typed(minutes(30)))
        );
        assert_eq!(
            time_left(Some(minutes(240)), minutes(300), Some(minutes(0))),
            Some(TimeLeft::Typed(minutes(0)))
        );
        assert_eq!(
            time_left(None, minutes(90), Some(minutes(45))),
            Some(TimeLeft::Typed(minutes(45)))
        );
        // Nothing to work it out from.
        assert_eq!(time_left(None, minutes(90), None), None);
        assert_eq!(
            time_left(Some(minutes(240)), minutes(80), None).map(TimeLeft::minutes),
            Some(minutes(160))
        );
    }

    #[test]
    fn time_03_the_vectors_hold_every_case_with_the_answer_of_the_rule() {
        let file: Value = serde_json::from_str(&time_left_file().unwrap()).unwrap();
        let cases = file["cases"].as_array().unwrap();
        assert_eq!(cases.len(), CASES.len());
        assert_eq!(
            cases[3],
            json!({"estimated": 240, "spent": 80, "typed": null, "left": 160})
        );
        assert_eq!(
            cases[0],
            json!({"estimated": null, "spent": 0, "typed": null, "left": null})
        );
    }
}
