//! Dependencies (PLAN-02, ADR-0043, decision 4): the relations of a kind with a date effect, and
//! the one rule about them, that they never close a loop.
//!
//! A dependency is no record of its own: it is a relation whose kind has a date effect, read from
//! the same rows as every other relation. The loop check is one walk (`bb_core::relation`) that
//! making a relation and giving a kind the effect both ask, and that the plan of the timeline can
//! ask for any two issues. The check runs on what is stored, once before the change for the
//! answer (which names the loop only as the issues the person may see) and again inside the change,
//! after the lock, so two changes at the same moment cannot close a loop between them. Loops that
//! were stored before the effect existed stay: only a loop that the new relation, or the kind that
//! is given the effect, would close is refused.

use bb_core::id::{IssueId, RelationKindId};
use bb_core::relation::{Relation, RelationKind, loop_closed_by};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};

/// The kinds that have a date effect: the kinds whose relations are dependencies.
pub(crate) fn effect_kinds(kinds: &[RelationKind]) -> Vec<RelationKindId> {
    kinds
        .iter()
        .filter(|kind| kind.date_effect.is_dependency())
        .map(|kind| kind.id)
        .collect()
}

/// The loop that `relation` would close, as the issues on it in order: none when its kind has no
/// date effect now, or when it closes none. On an open transaction, so it sees what the change
/// has written.
///
/// # Errors
/// The error of the store.
pub(crate) async fn loop_closed_by_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    relation: &Relation,
) -> Result<Option<Vec<IssueId>>, S::Error> {
    let kinds = S::relation_kinds_in(transaction).await?;
    let with_effect = effect_kinds(&kinds);
    if !with_effect.contains(&relation.kind) {
        return Ok(None);
    }
    let dependencies = S::dependencies_of_kinds_in(transaction, with_effect).await?;
    Ok(loop_closed_by(&dependencies, (relation.from, relation.to)))
}

/// A loop that the kind `kind` would close if it had the date effect, on the relations it has now
/// and the dependencies of the kinds that have the effect: the first one found, as the issues on
/// it in order. On an open transaction.
///
/// # Errors
/// The error of the store.
pub(crate) async fn loop_of_kind_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    kind: RelationKindId,
) -> Result<Option<Vec<IssueId>>, S::Error> {
    let kinds = S::relation_kinds_in(transaction).await?;
    let mut with_effect = effect_kinds(&kinds);
    if !with_effect.contains(&kind) {
        with_effect.push(kind);
    }
    let dependencies = S::dependencies_of_kinds_in(transaction, with_effect).await?;
    let own = S::dependencies_of_kinds_in(transaction, vec![kind]).await?;
    Ok(own
        .into_iter()
        .find_map(|each| loop_closed_by(&dependencies, each)))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The refusal for a loop: the issues on it in order, each as the issue ID the person may see,
    /// and an issue they may not see as none, so the loop never says that such an issue exists
    /// under any name (PRIV-13).
    ///
    /// # Errors
    /// The error of the store.
    pub(crate) async fn dependency_loop(
        &self,
        actor: &Actor,
        loop_of: &[IssueId],
    ) -> Result<AppError, AppError> {
        let issues = self.issues_named(loop_of).await?;
        let mut maker = self.line_maker().await?;
        let mut named = Vec::with_capacity(loop_of.len());
        for id in loop_of {
            let line = match issues.iter().find(|found| found.id == *id) {
                Some(issue) => self.line_of(actor, &mut maker, issue).await?,
                None => None,
            };
            named.push(line.map(|line| line.key));
        }
        Ok(AppError::DependencyLoop { issues: named })
    }
}
