//! The comments of the sample: how many, under which issues, by whom (a pick the maker turns into
//! a person), when, with which words, with a mention now and then, and which of them were edited.

use crate::issues_plan::PlannedIssue;
use crate::issues_time::WORK_DAY;
use crate::issues_words::{Area, IssueWords};
use crate::random::SeededRandom;

/// How far from the comment its edit may be at most.
const EDIT_WITHIN: i64 = 3 * WORK_DAY;
/// How far from its issue a comment may be at most.
const COMMENT_WITHIN: i64 = 60 * WORK_DAY;
/// The picks of people the maker turns into persons.
const PICKS: usize = 1 << 20;

/// The new text of a comment that was edited.
#[derive(Clone, Debug)]
pub(crate) struct PlannedEdit {
    pub(crate) at: i64,
    pub(crate) text: String,
}

/// A comment as planned.
#[derive(Clone, Debug)]
pub(crate) struct PlannedComment {
    pub(crate) issue: usize,
    pub(crate) at: i64,
    /// Who wrote it (and edits it), as a pick the maker turns into a person.
    pub(crate) lane: u32,
    /// The text; the marker [`MENTION`] stands where a person is mentioned.
    pub(crate) text: String,
    /// Who is mentioned, as a pick the maker turns into a person.
    pub(crate) mention: u32,
    pub(crate) edit: Option<PlannedEdit>,
}

/// How much more likely an issue is to be talked about: bugs and urgent issues draw comments.
fn weight(issue: &PlannedIssue) -> usize {
    1 + issue.priority as usize
}

/// Plans `count` comments under the issues, `edited_percent` of a hundred of them edited later.
pub(crate) fn plan_comments(
    count: usize,
    edited_percent: u32,
    issues: &[PlannedIssue],
    made_at: &[i64],
    end: i64,
    words: &IssueWords,
    random: &mut SeededRandom,
) -> Vec<PlannedComment> {
    if issues.is_empty() {
        return Vec::new();
    }
    // The running total of the weights, to draw an issue by weight.
    let mut running = Vec::with_capacity(issues.len());
    let mut sum = 0;
    for issue in issues {
        sum += weight(issue);
        running.push(sum);
    }
    let mut comments = Vec::with_capacity(count);
    for _ in 0..count {
        let draw = random.below(sum);
        let issue = running
            .partition_point(|total| *total <= draw)
            .min(issues.len() - 1);
        let area = Area::of_position(issues[issue].family);
        let room = (end - made_at[issue] - 20).clamp(0, COMMENT_WITHIN);
        let at = (made_at[issue] + 10 + random.skewed(room)).min(end);
        let text = text(words, area, random);
        let edit = random.chance(edited_percent).then(|| {
            let room = (end - at - 70).clamp(0, EDIT_WITHIN);
            let note = random
                .pick(&words.comments.edits)
                .cloned()
                .unwrap_or_default();
            PlannedEdit {
                at: (at + 60 + random.skewed(room)).min(end),
                text: format!("{text} {note}"),
            }
        });
        comments.push(PlannedComment {
            issue,
            at,
            lane: u32::try_from(random.below(PICKS)).unwrap_or(0),
            text,
            mention: u32::try_from(random.below(PICKS)).unwrap_or(0),
            edit,
        });
    }
    comments
}

/// What a comment says: a few sentences, sometimes with a mention.
fn text(words: &IssueWords, area: Area, random: &mut SeededRandom) -> String {
    let lists = &words.comments;
    let mut sentences = Vec::new();
    let mut add = |list: &[String], random: &mut SeededRandom| {
        let template = random.pick(list).cloned().unwrap_or_default();
        sentences.push(words.fill(&template, area, random));
    };
    add(&lists.first, random);
    if random.chance(65) {
        add(&lists.middle, random);
    }
    if random.chance(25) {
        add(&lists.middle, random);
    }
    if random.chance(30) {
        add(&lists.mentions, random);
    }
    if random.chance(50) {
        add(&lists.last, random);
    }
    sentences.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issues_words::MENTION;

    /// Whether a text of a comment has a mention in it.
    fn has_mention(text: &str) -> bool {
        text.contains(MENTION)
    }

    #[test]
    fn a_text_with_the_marker_has_a_mention_and_the_sentences_are_filled_in() {
        let words = IssueWords::bundled().unwrap();
        let mut random = SeededRandom::new(5);
        let texts: Vec<String> = (0..200)
            .map(|_| text(&words, Area::Web, &mut random))
            .collect();
        assert!(texts.iter().any(|text| has_mention(text)));
        assert!(texts.iter().any(|text| !has_mention(text)));
        for text in &texts {
            assert!(!text.replace(MENTION, "").contains('{'), "{text}");
            assert!(!text.is_empty());
        }
    }
}
