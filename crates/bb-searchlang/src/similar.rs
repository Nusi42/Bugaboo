//! Words that are close to a word the person wrote, for "Did you mean …?".

/// The most words offered.
const MOST_OFFERED: usize = 3;
/// How many letters may differ for a word to count as close.
const MOST_DIFFERENT: usize = 2;
/// The shortest start of a word that is enough to point to it.
const SHORTEST_START: usize = 3;

/// The number of single-letter changes (add, remove, change) from one text to the other, in lower
/// case; `None` when the texts differ in length by more than [`MOST_DIFFERENT`], because they then
/// differ by more letters than that, and the work for the number is not done.
fn distance(a: &str, b: &str) -> Option<usize> {
    let a: Vec<char> = a.to_lowercase().chars().collect();
    let b: Vec<char> = b.to_lowercase().chars().collect();
    if a.len().abs_diff(b.len()) > MOST_DIFFERENT {
        return None;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, right) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (diagonal + usize::from(left != right))
                .min(row[j] + 1)
                .min(above + 1);
            diagonal = above;
        }
    }
    Some(row[b.len()])
}

/// Up to three of `candidates` that are close to `word`, the closest first. A candidate is close
/// when it differs by at most two letters, or starts with the word.
#[must_use]
pub(crate) fn closest<'a>(
    word: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    let lower = word.to_lowercase();
    let mut close: Vec<(usize, &str)> = candidates
        .into_iter()
        .filter(|candidate| *candidate != word)
        .filter_map(|candidate| {
            let starts = lower.chars().count() >= SHORTEST_START
                && candidate.to_lowercase().starts_with(&lower);
            match distance(word, candidate) {
                Some(apart) if apart <= MOST_DIFFERENT => Some((apart, candidate)),
                // A word that starts with the text differs by the letters that follow it.
                _ if starts => Some((
                    candidate
                        .chars()
                        .count()
                        .saturating_sub(lower.chars().count()),
                    candidate,
                )),
                _ => None,
            }
        })
        .collect();
    close.sort_unstable();
    close
        .into_iter()
        .take(MOST_OFFERED)
        .map(|(_, candidate)| candidate.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typo_finds_the_field_and_a_far_word_finds_nothing() {
        let fields = ["priority", "assignedTo", "status", "statusGroup"];
        assert_eq!(closest("prio", fields), ["priority"]);
        assert_eq!(closest("Priority", fields), ["priority"]);
        assert_eq!(closest("stat", fields), ["status", "statusGroup"]);
        assert!(closest("banana", fields).is_empty());
    }

    #[test]
    fn texts_that_differ_in_length_by_more_than_two_are_not_counted() {
        assert_eq!(distance("status", "statuz"), Some(1));
        assert_eq!(distance("status", "statusGroup"), None);
        assert_eq!(distance("", "ab"), Some(2));
        assert_eq!(distance("", "abc"), None);
        let long = "x".repeat(100_000);
        assert_eq!(distance(&long, "x"), None);
    }
}
