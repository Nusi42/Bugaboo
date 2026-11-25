//! Where a card may go (PLAN-06, ADR-0031): what the workflow of the card allows the reader now,
//! read from the status changes its issue view already holds (`status_changes`, judged by
//! `bb_core::workflow::judge`), and for a column of choices whether the reader may change the field.
//! No rule of the workflow is written again: the move itself is the status change or the field
//! change that exists, which judges everything once more when it is made.

use bb_core::fields::{Field, FieldKey};
use bb_core::names::same_name;
use bb_core::workflow::WorkflowContent;
use bb_policy::Evaluation;

use super::columns::{Column, ColumnsBy};
use super::types::{CardMove, MoveRefused};
use crate::issues::IssueView;

/// A move to a column of a status or a status group: the first change the reader may make that
/// enters it; else why not.
fn status_move(
    workflow: &WorkflowContent,
    view: &IssueView,
    column: &Column,
    at: usize,
) -> CardMove {
    let enters = |name: &str, group| match column.status_group {
        Some(wanted) => group == wanted,
        None => column
            .key
            .as_deref()
            .is_some_and(|key| same_name(key, name)),
    };
    if let Some(change) = view
        .status_changes
        .iter()
        .find(|change| enters(&change.to.name, change.to.group))
    {
        return CardMove {
            column: at,
            to: Some(change.to.key.clone()),
            needs: change.needs.clone(),
            refused: None,
        };
    }
    let has_status = workflow
        .statuses
        .iter()
        .any(|status| enters(&status.name, status.group));
    CardMove {
        column: at,
        to: None,
        needs: Vec::new(),
        refused: Some(if has_status {
            MoveRefused::NoSuchChange
        } else {
            MoveRefused::NoSuchStatus
        }),
    }
}

/// Where a card may go: one move for each column it is not in. `changeable` are the fields the
/// reader may change on the issue; a reader who may not read the status of the card is told no
/// move by status at all, so the moves tell nothing the card does not show.
pub(crate) fn card_moves(
    (by, columns): (ColumnsBy, &[Column]),
    (held, view): (&Evaluation, &IssueView),
    workflow: &WorkflowContent,
    changeable: &[FieldKey],
    (own, issue_type): (usize, bb_core::id::IssueTypeId),
) -> Vec<CardMove> {
    let reads_status = held.field_level_for(Some(issue_type), Field::Status)
        >= bb_core::access::FieldLevel::CanSee;
    columns
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != own)
        .filter_map(|(at, column)| match by {
            ColumnsBy::Status | ColumnsBy::StatusGroup => {
                reads_status.then(|| status_move(workflow, view, column, at))
            }
            ColumnsBy::Choice(field) => Some(CardMove {
                column: at,
                to: None,
                needs: Vec::new(),
                refused: (!changeable.contains(&FieldKey::Own(field)))
                    .then_some(MoveRefused::CannotChange),
            }),
        })
        .collect()
}
