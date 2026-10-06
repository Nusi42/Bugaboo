//! Giving what a deleted role gave to another role (SEC-08): deleting a role that somebody has asks
//! which role takes its place, and every row that gave the old role gives the new one instead.

use bb_core::access::Assignment;
use bb_core::id::{AssignmentId, IdSource, RoleId, UserId};
use jiff::Timestamp;

use super::assignments::same_subject;
use crate::access::ensure_may_give_in;
use crate::auth::AuthStore;

/// Makes, for every row that gives a role, a row for the same person or group, place and reach
/// that gives `replacement`, unless such a row is there already, and returns the rows it made (each
/// is a permission change for the security log, ADR-0025 decision 9). Nobody gives a role they do
/// not hold themselves, so `giver` must hold the replacement wherever it is given. It runs inside
/// the change, after the policy lock.
///
/// # Errors
/// [`bb_policy::guards::Refusal::TooMuch`] (as the error of the store) when the giver holds less
/// than the replacement somewhere, or the error of the store.
pub(super) async fn give_to_replacement<S: AuthStore>(
    transaction: &mut S::Transaction,
    giver: UserId,
    rows: &[Assignment],
    replacement: RoleId,
    now: Timestamp,
    ids: &mut impl IdSource,
) -> Result<Vec<Assignment>, S::Error> {
    let mut made = Vec::new();
    for old in rows {
        let new = Assignment {
            id: AssignmentId::generate(ids).map_err(|_| S::aborted())?,
            role: Some(replacement),
            given_at: now,
            ..old.clone()
        };
        ensure_may_give_in::<S>(transaction, giver, &new).await?;
        let there = S::assignments_on_in(transaction, new.project).await?;
        let already = there.iter().any(|row| {
            same_subject(&new, row)
                && row.role == Some(replacement)
                && row.applies_to == new.applies_to
        });
        if !already {
            S::insert_assignment(transaction, &new).await?;
            made.push(new);
        }
    }
    Ok(made)
}
