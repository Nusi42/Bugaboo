//! The standing rules of a removed person go with what is part of them, on both databases
//! (ADR-0044, decision 6): the issues they found and their open proposals go in the same change;
//! a decided proposal is history and stays. The rules of another person keep theirs.

use bb_app::agent_work::GrantStore as _;
use bb_app::auth::AccountStore as _;
use bb_app::ports::{ChangeStore as _, PersonData as _};
use bb_core::agent_records::{Proposal, ProposalState, StandingRule};
use bb_core::agent_work::RuleField;
use bb_core::id::{AgentNoteId, IssueId, UserId};
use bb_testing::{TestResult, each_database};

use super::tests_agent::an_agent;
use super::tests_agent_work::{ENDS, Given, Made, given, rule_that_found};
use crate::AuthStorage;
use crate::test_support::{at, event, person};

/// A proposal of `rule` on `issue` in `state`.
fn proposal(
    made: &mut Made,
    rule: &StandingRule,
    issue: IssueId,
    state: ProposalState,
) -> Proposal {
    Proposal {
        id: AgentNoteId::from_random(made.bytes()),
        rule: rule.id,
        agent: rule.agent,
        issue,
        field: RuleField::Type,
        before: serde_json::Value::Null,
        after: serde_json::json!("x"),
        state,
        decided_by: None,
        made_at: at(3),
    }
}

/// How many rows of `kind` the walk finds for `person`.
async fn walked(
    storage: &AuthStorage,
    person: UserId,
    kind: &str,
) -> Result<u64, crate::StoreError> {
    Ok(storage
        .kinds_of(person)
        .await?
        .into_iter()
        .find(|found| found.kind == kind)
        .map_or(0, |found| found.rows))
}

#[tokio::test]
async fn priv_04_the_rules_of_a_removed_person_go_with_their_finds_and_open_proposals() -> TestResult
{
    each_database("priv_04_rules_and_their_parts", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let issue = data.issues[0].id;
        let rule = rule_that_found(
            storage,
            &mut made,
            (owner, fixer),
            &data,
            &[issue],
            at(ENDS),
        )
        .await?;
        let other = bb_core::person::Person {
            // An ID of its own: the sequence of the test began where the one of `given` did.
            id: UserId::from_random([0x5a; 16]),
            ..person(&mut made.source, "other@example.test")
        };
        let stored_other = other.clone();
        storage
            .record_change(&event(&mut made.source, 77), move |tx| {
                Box::pin(async move {
                    AuthStorage::insert_person(tx, &stored_other, "hash", at(0)).await
                })
            })
            .await?;
        let helper = an_agent(storage, other.id, "Helper", 9).await?;
        let theirs = rule_that_found(
            storage,
            &mut made,
            (other.id, helper),
            &data,
            &[issue],
            at(ENDS),
        )
        .await?;
        let proposals = [
            proposal(&mut made, &rule, issue, ProposalState::Open),
            proposal(&mut made, &rule, issue, ProposalState::Discarded),
            proposal(&mut made, &theirs, issue, ProposalState::Open),
        ];
        made.step += 1;
        let step = made.step;
        let kept = proposals.to_vec();
        storage
            .record_change(&event(&mut made.source, step), move |tx| {
                Box::pin(async move {
                    for each in &kept {
                        AuthStorage::insert_proposal(tx, each).await?;
                    }
                    Ok(())
                })
            })
            .await?;
        // The walk finds the finds and the proposals of the rule as the person's.
        assert_eq!(walked(storage, owner, "rule_match").await?, 1);
        assert_eq!(walked(storage, owner, "agent_proposal").await?, 2);
        made.step += 1;
        let step = made.step;
        storage
            .record_change(&event(&mut made.source, step), move |tx| {
                Box::pin(async move { AuthStorage::delete_rules_of(tx, owner).await.map(|_| ()) })
            })
            .await?;
        assert!(storage.matches_of(rule.id, None, 10).await?.is_empty());
        let left = storage.proposals_on(issue, None, None, 10).await?;
        let mut states: Vec<(bool, ProposalState)> = left
            .iter()
            .map(|each| (each.rule == rule.id, each.state))
            .collect();
        states.sort_by_key(|(mine, _)| *mine);
        // Only the decided proposal of the rule stays, as history, beside the other person's.
        assert_eq!(
            states,
            [
                (false, ProposalState::Open),
                (true, ProposalState::Discarded)
            ]
        );
        assert_eq!(walked(storage, owner, "standing_rule").await?, 0);
        // The other person's rule keeps its finds.
        assert_eq!(storage.matches_of(theirs.id, None, 10).await?.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_a_find_of_a_rule_that_is_gone_or_ended_is_not_written() -> TestResult {
    each_database("priv_04_find_of_a_rule_gone", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let issue = data.issues[0].id;
        let rule =
            rule_that_found(storage, &mut made, (owner, fixer), &data, &[], at(ENDS)).await?;
        let gone = bb_core::id::StandingRuleId::from_random(made.bytes());
        // As the job of the rules writes what it found, in a change after it read the rule: the
        // rule a removal deleted meanwhile gets nothing; the open rule gets its find.
        made.step += 1;
        let step = made.step;
        let written = storage
            .record_change(&event(&mut made.source, step), move |tx| {
                Box::pin(async move {
                    let find = |rule| bb_core::agent_records::RuleMatch {
                        rule,
                        issue,
                        at: at(2),
                    };
                    let lost = AuthStorage::insert_match(tx, &find(gone)).await?;
                    let kept = AuthStorage::insert_match(tx, &find(rule.id)).await?;
                    Ok((lost, kept))
                })
            })
            .await?;
        assert_eq!(written, (false, true));
        assert!(storage.matches_of(gone, None, 10).await?.is_empty());
        assert_eq!(storage.matches_of(rule.id, None, 10).await?.len(), 1);
        Ok(())
    })
    .await
}
