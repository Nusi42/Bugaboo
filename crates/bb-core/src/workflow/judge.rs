//! The judge of a status change (WF-03, WF-04, ISS-09, ADR-0031 decision 3): the one function that
//! says whether a person may make a status change on an issue now, and what it needs.
//!
//! The issue page, the status button, *Change many issues* and its preview, *Try it*, the API and
//! later the assistant all ask it. It decides from plain facts (who the person is for this issue,
//! what they may do with each field, which fields have a value), so every caller gives it the same
//! facts and gets the same answer.
//!
//! Two rules run through it:
//! - **A rule never gives a right.** The judge only narrows: a person who may not change the status
//!   of the issue is turned away whatever *Who can do it* says.
//! - **A hidden field is never named.** A rule that names a field the person may not see turns the
//!   change down without naming any field, whether the field has a value or not, so the answer
//!   tells nothing about it (SEC-06).

use super::{StatusKey, StatusRules, Test, WorkflowContent};
use crate::ErrorCode;
use crate::access::FieldLevel;
use crate::fields::FieldKey;
use crate::id::{GroupId, RoleId};

words! {
    /// Why a status change is not allowed.
    pub enum Why {
        /// The workflow has no status change from the status the issue is in to that one.
        NoSuchChange => "no-such-change",
        /// The person may not change the status of the issue at all.
        NotForYou => "not-for-you",
        /// The person may change the status of the issue, but *Who can do it* does not name them
        /// for this status change.
        NotNamed => "not-named",
        /// *Only if …* is not true.
        OnlyIf => "only-if",
        /// The change needs a field that the person cannot fill in.
        CannotFill => "cannot-fill",
    }
}

words_in_api!(Why, "Why a status change is not allowed.");

impl Why {
    /// The error code a refusal for this reason is answered with.
    #[must_use]
    pub const fn error(self) -> ErrorCode {
        match self {
            Self::NoSuchChange => ErrorCode::WorkflowNoSuchChange,
            Self::NotForYou => ErrorCode::WorkflowNotForYou,
            Self::NotNamed => ErrorCode::WorkflowNotNamed,
            Self::OnlyIf => ErrorCode::WorkflowOnlyIf,
            Self::CannotFill => ErrorCode::WorkflowCannotFill,
        }
    }
}

/// Why a status change is not allowed, and the field it is about when the person may know it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Because {
    /// The reason.
    pub why: Why,
    /// The field, only when the person may see it.
    pub field: Option<FieldKey>,
}

/// What the judge says about a status change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The person may make the change as it is.
    Allowed,
    /// The person may make the change once these fields have a value: *Must be filled in* fields
    /// that are empty, and *Ask for* fields that the request did not carry.
    Needs(Vec<FieldKey>),
    /// The person may not make the change.
    NotAllowed(Because),
}

/// Who makes the change, for the issue at hand.
#[derive(Clone, Copy, Debug)]
pub struct Changer<'a> {
    /// Whether the person may change the status of the issue at all (*Change issues*, or *Change
    /// own issues* on an issue they reported, and a field level that lets them change *Status*).
    pub may_change_status: bool,
    /// The roles the person holds where the issue is.
    pub roles: &'a [RoleId],
    /// The groups the person is in.
    pub groups: &'a [GroupId],
    /// Whether the issue is assigned to the person.
    pub assigned: bool,
    /// Whether the person reported the issue.
    pub reporter: bool,
}

/// What the judge needs to know about the fields of the issue, as it will be after the change.
pub trait FieldFacts {
    /// What the person may do with the field on this issue.
    fn level(&self, field: FieldKey) -> FieldLevel;
    /// Whether the field has a value after the change.
    fn filled(&self, field: FieldKey) -> bool;
    /// Whether the request carries the field, changed or not.
    fn given(&self, field: FieldKey) -> bool;
}

fn refused(why: Why, field: Option<FieldKey>) -> Verdict {
    Verdict::NotAllowed(Because { why, field })
}

fn named_by_who(rules: &StatusRules, changer: &Changer<'_>) -> bool {
    let who = &rules.who;
    who.is_everybody()
        || (who.assigned && changer.assigned)
        || (who.reporter && changer.reporter)
        || who.roles.iter().any(|role| changer.roles.contains(role))
        || who
            .groups
            .iter()
            .any(|group| changer.groups.contains(group))
}

fn only_if(rules: &StatusRules, facts: &dyn FieldFacts) -> Option<Verdict> {
    rules
        .only_if
        .iter()
        .find(|condition| {
            let filled = facts.filled(condition.field);
            match condition.test {
                Test::Filled => !filled,
                Test::Empty => filled,
            }
        })
        .map(|condition| refused(Why::OnlyIf, Some(condition.field)))
}

/// The fields the change needs, or the reason it cannot be made: a field of *Must be filled in* or
/// *Ask for* has to be one the person can change.
fn needed(rules: &StatusRules, facts: &dyn FieldFacts) -> Result<Vec<FieldKey>, Verdict> {
    let mut needs: Vec<FieldKey> = Vec::new();
    let wanted = rules
        .must_fill
        .iter()
        .filter(|field| !facts.filled(**field))
        .chain(rules.ask_for.iter().filter(|field| !facts.given(**field)));
    for field in wanted {
        if facts.level(*field) < FieldLevel::CanChange {
            return Err(refused(Why::CannotFill, Some(*field)));
        }
        if !needs.contains(field) {
            needs.push(*field);
        }
    }
    Ok(needs)
}

/// Whether the person may change the issue from `from` to `to`, given the workflow that counts for
/// the issue.
///
/// An issue whose status the workflow does not have (the status was dropped, or the issue moved to
/// a workflow that has no such status) may go only to the starting status of the workflow, and to
/// no other: nothing is allowed without the rules.
#[must_use]
pub fn judge(
    workflow: &WorkflowContent,
    from: &StatusKey,
    to: &StatusKey,
    changer: &Changer<'_>,
    facts: &dyn FieldFacts,
) -> Verdict {
    if !changer.may_change_status {
        return refused(Why::NotForYou, None);
    }
    if from == to {
        return Verdict::Allowed;
    }
    if workflow.status(from).is_none() {
        return if workflow.initial() == Some(to) {
            Verdict::Allowed
        } else {
            refused(Why::NoSuchChange, None)
        };
    }
    let Some(change) = workflow.change(from, to) else {
        return refused(Why::NoSuchChange, None);
    };
    let rules = &change.rules;
    if !named_by_who(rules, changer) {
        return refused(Why::NotNamed, None);
    }
    // A rule that names a hidden field turns the change down before any value is looked at, and
    // names no field, so neither the answer nor the order of the checks tells anything about it.
    if rules
        .fields()
        .into_iter()
        .any(|field| facts.level(field) < FieldLevel::CanSee)
    {
        return refused(Why::CannotFill, None);
    }
    if let Some(verdict) = only_if(rules, facts) {
        return verdict;
    }
    match needed(rules, facts) {
        Ok(needs) if needs.is_empty() => Verdict::Allowed,
        Ok(needs) => Verdict::Needs(needs),
        Err(verdict) => verdict,
    }
}

#[cfg(test)]
mod tests;
