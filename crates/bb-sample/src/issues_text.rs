//! The words of one planned issue: its priority, its long text and the values of its own fields.

use bb_core::issue::Priority;
use bb_dev_words::sample::FieldPlan;

use crate::issues_plan::FieldPick;
use crate::issues_words::{IssueWords, Templates};
use crate::random::SeededRandom;

/// The priority of a new issue: bugs are more often urgent.
pub(crate) fn priority(bug: bool, random: &mut SeededRandom) -> Priority {
    let (low, normal, high) = if bug { (10, 60, 88) } else { (20, 80, 97) };
    match random.below(100) {
        roll if roll < low => Priority::Low,
        roll if roll < normal => Priority::Normal,
        roll if roll < high => Priority::High,
        _ => Priority::Critical,
    }
}

/// The long text of an issue, in Markdown.
pub(crate) fn describe(
    words: &IssueWords,
    templates: &Templates,
    bug: bool,
    area: &str,
    random: &mut SeededRandom,
) -> String {
    let fill = |list: &[String], random: &mut SeededRandom| {
        let template = random.pick(list).cloned().unwrap_or_default();
        words.fill(&template, area, random)
    };
    if bug {
        let happens = fill(&templates.happens, random);
        let steps: Vec<String> = (1..=3)
            .map(|number| format!("{number}. {}", fill(&templates.steps, random)))
            .collect();
        let expected = fill(&templates.expected, random);
        format!(
            "## What happens\n\n{happens}\n\n## Steps\n\n{}\n\n## What should happen\n\n{expected}\n",
            steps.join("\n")
        )
    } else {
        let about = fill(&templates.about, random);
        let more = if random.chance(50) {
            format!("\n\n{}", fill(&templates.about, random))
        } else {
            String::new()
        };
        format!("{about}{more}\n")
    }
}

/// The value of an own field for an issue of this type in this top-level project: nothing when
/// the field is not used there or the issue has no value.
pub(crate) fn field_pick(
    field: &FieldPlan,
    type_name: &str,
    family: &str,
    words: &IssueWords,
    random: &mut SeededRandom,
) -> Option<FieldPick> {
    let for_type = field.issue_types.is_empty() || field.issue_types.iter().any(|n| n == type_name);
    let for_project = field.projects.is_empty() || field.projects.iter().any(|n| n == family);
    if !(for_type && for_project) {
        return None;
    }
    match field.kind.as_str() {
        // The mild choices come first in the list and are the common ones.
        "pick-one" => {
            let at = random.skewed(i64::try_from(field.choices.len()).unwrap_or(0));
            Some(FieldPick::Choices(vec![usize::try_from(at).unwrap_or(0)]))
        }
        "pick-several" if random.chance(70) => {
            let mut chosen = vec![random.below(field.choices.len())];
            for _ in 0..random.below(3) {
                let more = random.below(field.choices.len());
                if !chosen.contains(&more) {
                    chosen.push(more);
                }
            }
            chosen.sort_unstable();
            Some(FieldPick::Choices(chosen))
        }
        "text" if random.chance(40) => random
            .pick(words.customers())
            .map(|name| FieldPick::Text(name.clone())),
        _ => None,
    }
}
