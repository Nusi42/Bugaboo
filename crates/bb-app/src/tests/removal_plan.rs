//! The plan of *Remove a person* against the in-memory store (PRIV-04, ADR-0044, decision 6): a
//! kind of the account goes by the rows that are the person's own, and the rows that name them
//! another way stay, shown as *Former user*.

use bb_core::id::UserId;
use bb_core::personal::{Entity, EntityMarks, Link};

use super::fake_exports::WalkedRow;
use super::scenarios::token;
use super::world::{Outcome, World};
use crate::privacy::export::title_of;
use crate::privacy::removal::{RemovalPlanQuery, removal_plan};
use crate::privacy::walk::FoundRow;

/// A row of `E` that names `person` as `link`, as the walk finds it.
fn row<E: Entity>(person: UserId, link: Link) -> WalkedRow {
    WalkedRow {
        person,
        kind: E::MARKS.name().to_owned(),
        marks: E::MARKS,
        row: FoundRow {
            links: vec![link],
            cells: Vec::new(),
        },
    }
}

/// The rows a list of the plan shows for the kind of `marks`.
fn rows_of(list: &[crate::privacy::removal::PlanKind], marks: &EntityMarks) -> u64 {
    list.iter()
        .filter(|kind| kind.title == title_of(marks))
        .map(|kind| kind.rows)
        .sum()
}

#[tokio::test]
async fn priv_04_the_plan_keeps_what_names_the_person_only_by_another_link() -> Outcome {
    use bb_core::agent::Agent;
    use bb_core::agent_request::AgentRequest;
    use bb_core::notification::Notification;
    let world = World::new().await?;
    let bob = world.person("bob@example.test").await?.person.id;
    {
        let mut shared = world.store.lock();
        let walk = &mut shared.state.walk;
        // A notification for Bob goes; one he caused for somebody else stays.
        walk.push(row::<Notification>(bob, Link::Recipient));
        walk.push(row::<Notification>(bob, Link::Actor));
        walk.push(row::<Notification>(bob, Link::Actor));
        // His own agent goes; an agent of somebody else that he made or is offered stays.
        walk.push(row::<Agent>(bob, Link::Holder));
        walk.push(row::<Agent>(bob, Link::Author));
        // A request of a program for agents asked of Bob goes with his account.
        walk.push(row::<AgentRequest>(bob, Link::Holder));
    }
    let plan = removal_plan(
        &world.ctx(),
        RemovalPlanQuery {
            session_token: token(&world),
            person: bob,
        },
    )
    .await?
    .into_value();
    assert_eq!(rows_of(&plan.deleted_now, &Notification::MARKS), 1);
    assert_eq!(rows_of(&plan.kept_as_former_user, &Notification::MARKS), 2);
    assert_eq!(rows_of(&plan.deleted_now, &Agent::MARKS), 1);
    assert_eq!(rows_of(&plan.kept_as_former_user, &Agent::MARKS), 1);
    assert_eq!(rows_of(&plan.deleted_now, &AgentRequest::MARKS), 1);
    assert_eq!(rows_of(&plan.kept_as_former_user, &AgentRequest::MARKS), 0);
    // Each kind in its plain name, which the window writes before a colon and the count.
    let titles: Vec<&str> = plan
        .deleted_now
        .iter()
        .map(|kind| kind.title.as_str())
        .collect();
    assert_eq!(titles, ["Notifications", "Agents", "Requests for agents"]);
    Ok(())
}
