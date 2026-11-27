//! The plan of a removal and the parts of the person's records, through the real wiring, on both
//! databases (ADR-0044, decision 6): a row that is a part of a record the removal deletes is
//! counted with that record, even when it also names the person in its own text; a part of
//! somebody else's record that names the person stays, and the plan says so.

use bb_app::access::StartingRole;
use bb_app::agent_work::GrantStore as _;
use bb_app::context::SystemIds;
use bb_app::ports::ChangeStore;
use bb_core::agent_records::{Proposal, ProposalState};
use bb_core::agent_work::RuleField;
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, RecordKind};
use bb_core::id::{
    AgentId, AgentNoteId, EntityId, EventId, IdSource as _, IssueId, StandingRuleId, UserId,
};
use bb_store::AuthStorage;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::data_export_parts_http::plan_list;
use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, id_of};
use super::removal_http::switch_off;
use super::rig::Rig;
use super::rules_hand_over_http::{agent_with_limit, rule_over};

/// The title of proposals in the plan.
const PROPOSALS: &str = "Proposed changes";

/// A team member's agent and standing rule over ACME, by their IDs.
async fn agent_and_rule(
    site: &Site<'_>,
    person: &mut Browser,
) -> Result<(AgentId, StandingRuleId), Failure> {
    let (agent, _) = agent_with_limit(person, 5).await?;
    let rule = rule_over(site, person, &agent, ("", "Everything"), false).await?;
    Ok((
        agent.parse().map_err(|_| "no agent ID")?,
        rule.parse().map_err(|_| "no rule ID")?,
    ))
}

/// Stores an open proposal of `rule` on `issue` whose values name `named`.
async fn proposal_naming(
    rig: &Rig,
    (agent, rule): (AgentId, StandingRuleId),
    issue: IssueId,
    named: UserId,
) -> Result<(), Failure> {
    let mut ids = SystemIds;
    let stored = Proposal {
        id: AgentNoteId::from_random(ids.random()),
        rule,
        agent,
        issue,
        field: RuleField::Type,
        before: json!([]),
        after: json!([named.to_string()]),
        state: ProposalState::Open,
        decided_by: None,
        made_at: rig.clock_now(),
    };
    let event = DomainEvent {
        id: EventId::generate(&mut ids)?,
        time: rig.clock_now(),
        actor: Actor::Person(named),
        channel: Channel::Api,
        entity: EntityRef {
            kind: EntityKind::from(RecordKind::StandingRule),
            id: EntityId::from(rule),
        },
        changes: Vec::new(),
        reason: None,
        run: None,
    };
    ChangeStore::record_change(&rig.storage, &event, move |transaction| {
        Box::pin(async move { AuthStorage::insert_proposal(transaction, &stored).await })
    })
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[tokio::test]
async fn priv_04_the_plan_counts_a_part_with_its_record_even_when_it_names_the_person() -> TestResult
{
    each_database("priv_04_parts_in_the_plan", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        rig.turn_ai_apps_and_agents_on().await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tess, tess_id) = site.person(StartingRole::TeamMember, "tess").await?;
        let (mut uma, _) = site.person(StartingRole::TeamMember, "uma").await?;
        let hers = agent_and_rule(&site, &mut tess).await?;
        let theirs = agent_and_rule(&site, &mut uma).await?;
        let issue: IssueId = id_of(&site.create(&mut ada, "Printer jams").await?)?
            .parse()
            .map_err(|_| "no issue ID")?;
        // Three open proposals of Tess's own rule name her; one of Uma's rule names her too.
        for _ in 0..3 {
            proposal_naming(&rig, hers, issue, tess_id).await?;
        }
        proposal_naming(&rig, theirs, issue, tess_id).await?;
        switch_off(&mut ada, tess_id).await?;
        let plan = ada
            .send(
                "GET",
                &format!("/api/v1/people/{tess_id}/removal-plan"),
                None,
            )
            .await?;
        let rows = |list: &str| {
            plan_list(&plan.body, list)
                .into_iter()
                .find(|(title, _)| title == PROPOSALS)
                .map(|(_, rows)| rows)
        };
        // Her own rule's proposals go with the rule, which the plan counts; Uma's stays.
        assert_eq!(rows("deleted_now"), None, "{}", plan.body);
        assert_eq!(rows("kept_as_former_user"), Some(1), "{}", plan.body);
        Ok(())
    })
    .await
}
