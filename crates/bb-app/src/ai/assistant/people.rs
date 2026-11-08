//! Nothing about people is decided by the assistant (COMP-10, ADR-0029 decision 9).
//!
//! A proposal that would change *Assigned to* is shown only when the question of the person asked
//! for it and named exactly these people, as the person sees them. The model is told never to
//! choose, and this is the rule that holds when it does anyway. It fails closed: a question that
//! does not use the word, or does not name every person, gets no such proposal.

use bb_core::fields::{ASSIGNED_TO_PEOPLE, Field};
use bb_core::id::UserId;
use bb_policy::{PeopleContext, PersonRef, Place};
use serde_json::Value;

use crate::access::Actor;
use crate::ai::tools::ProposedChange;
use crate::auth::{AppError, AuthStore, Ctx};

/// The word of the question that asks for an assignment. It is a stem, so *assign*, *assigned* and
/// *assignment* all count. A question in another language that does not hold it gets no such
/// proposal (M7-F51).
const ASSIGN_STEM: &str = "assign";

/// The change of *Assigned to* among the changes, if there is one.
fn assignment(changes: &[ProposedChange]) -> Option<&ProposedChange> {
    changes
        .iter()
        .find(|change| change.field == Field::AssignedTo.code())
}

/// The people an after-value names by ID.
fn people_in(value: &Value, found: &mut Vec<UserId>) {
    match value {
        Value::String(text) => found.extend(text.parse::<UserId>()),
        Value::Array(items) => items.iter().for_each(|item| people_in(item, found)),
        Value::Object(members) => members.values().for_each(|item| people_in(item, found)),
        _ => {}
    }
}

/// Whether `name` stands in `text` as whole words: not inside a longer word, so *Al* is not found
/// in *all* and *Bob* is not found in *Bobby*. Both are in lower case.
fn names(text: &str, name: &str) -> bool {
    !name.is_empty()
        && text.match_indices(name).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + name.len()..].chars().next();
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
        })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether the proposal may be shown: the call names nobody for *Assigned to* (a person given
    /// by the rule of the project, like the default assignee of a new issue, is the project's choice
    /// and not the AI's), or the question asked for it and wrote the name of every person the call
    /// puts there, as the person sees that name at the place of the issue.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(super) async fn assignment_was_asked_for(
        &self,
        actor: &Actor,
        question: &str,
        place: Place,
        input: &Value,
        changes: &[ProposedChange],
    ) -> Result<bool, AppError> {
        // Only a person the call itself names is the AI's choice.
        if input.get("assigned_to").is_none_or(Value::is_null) {
            return Ok(true);
        }
        let Some(change) = assignment(changes) else {
            return Ok(true);
        };
        let question = question.to_lowercase();
        if !question.contains(ASSIGN_STEM) {
            return Ok(false);
        }
        let mut people = Vec::new();
        if let Some(after) = &change.after {
            people_in(after, &mut people);
        }
        // A value that puts someone there but names nobody we can read is not trusted. An empty
        // value takes the people away: the question asked for it with the word, and there is no
        // one to name.
        let puts_someone = change
            .after
            .as_ref()
            .is_some_and(|after| !(after.is_null() || after.as_array().is_some_and(Vec::is_empty)));
        if puts_someone && people.is_empty() {
            return Ok(false);
        }
        let viewer = self.evaluation_of(actor, place).await?;
        for person in people {
            let shown = self
                .person_ref(&viewer, ASSIGNED_TO_PEOPLE, PeopleContext::Ordinary, person)
                .await?;
            let PersonRef::Name { name, .. } = shown else {
                return Ok(false);
            };
            if !names(&question, &name.to_lowercase()) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::names;

    #[test]
    fn comp_10_a_name_counts_only_as_a_whole_word() {
        assert!(names("assign it to al", "al"));
        assert!(names("assign it to al, please", "al"));
        assert!(names("assign it to bob builder.", "bob builder"));
        assert!(names("assign it to bob's", "bob"));
        assert!(!names("assign all of it to the team", "al"));
        assert!(!names("assign it to bobby", "bob"));
        assert!(!names("assign it to abob", "bob"));
        assert!(!names("assign it", ""));
    }
}
