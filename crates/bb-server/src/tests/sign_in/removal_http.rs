//! *Remove a person* over HTTP on SQLite and PostgreSQL (PRIV-04, ADR-0044, decisions 6 and 7):
//! only a switched-off person is removed; afterwards the person is *Former user* everywhere, the
//! walk finds nothing of theirs but what is kept, the changes keep no name or address, the shared
//! search shows *Former user* as its maker, the work days stay under the legal-hold name, and the
//! comments stay or go as asked. Nothing undoes it.

use bb_app::people::StandingStore as _;
use bb_app::ports::PersonData as _;
use bb_core::id::UserId;
use bb_core::person::{HoldReason, Standing};
use bb_core::removal::{AtRemoval, at_removal, kept_when_shared};
use bb_policy::StartingRole;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, key_of, ok};
use super::removal_plan_http::bob_leaves_traces;
use super::rig::Rig;
use super::working_hours_http::MINE;

/// The name and the address of the person who is removed.
const NAME: &str = "Bob Builder";
const ADDRESS: &str = "bob@example.test";
/// The handle his name gave him.
const HANDLE: &str = "bob.builder";

/// Removes `person` as `browser`, with the `body` of the request.
pub(super) async fn remove(
    browser: &mut Browser,
    person: UserId,
    body: Value,
) -> Result<Value, Failure> {
    let reply = browser
        .send(
            "POST",
            &format!("/api/v1/people/{person}/remove"),
            Some(body),
        )
        .await?;
    Ok(json!({"status": reply.status.as_u16(), "code": reply.code(), "body": reply.body}))
}

/// Switches `person` off as `browser`.
pub(super) async fn switch_off(browser: &mut Browser, person: UserId) -> Result<(), Failure> {
    let reply = browser
        .send("POST", &format!("/api/v1/people/{person}/switch-off"), None)
        .await?;
    assert!(reply.status.is_success(), "{}", reply.body);
    Ok(())
}

/// Writes a comment on `issue` as `browser`.
async fn comment(browser: &mut Browser, issue: &str, text: &str) -> Result<(), Failure> {
    ok(&browser
        .send(
            "POST",
            &format!("/api/v1/issues/{issue}/comments"),
            Some(json!({"text": text})),
        )
        .await?)?;
    Ok(())
}

/// The comments of `issue`, as `browser` sees them.
async fn comments(browser: &mut Browser, issue: &str) -> Result<String, Failure> {
    Ok(ok(&browser
        .send(
            "GET",
            &format!("/api/v1/issues/{issue}/history?tab=comments"),
            None,
        )
        .await?)?
    .to_string())
}

/// The kinds that still hold rows of `person` by a link a removal deletes them by: none may be left
/// but the shared searches. Rows that name the person another way (a notification they caused for
/// somebody else) stay.
async fn left_to_delete(rig: &Rig, person: UserId) -> Result<Vec<String>, Failure> {
    let mut left = Vec::new();
    for kind in rig.storage.kinds_of(person).await? {
        let AtRemoval::Deleted { by } = at_removal(&kind.marks) else {
            continue;
        };
        if !kept_when_shared(&kind.marks)
            && rig.storage.rows_linked(person, &kind.kind, by).await? > 0
        {
            left.push(kind.kind);
        }
    }
    Ok(left)
}

/// Every cell of the events that name `person`, as text.
pub(super) async fn events_text(rig: &Rig, person: UserId) -> Result<String, Failure> {
    let mut cells = Vec::new();
    let mut from = Some(0);
    while let Some(start) = from {
        let page = rig.storage.person_rows(person, "event", start).await?;
        cells.extend(page.rows.iter().map(|row| format!("{:?}", row.cells)));
        from = page.next;
    }
    Ok(cells.join("\n"))
}

#[tokio::test]
async fn priv_04_a_removed_person_is_former_user_everywhere_but_the_work_days_for_hr() -> TestResult
{
    each_database("priv_04_removal_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let issue = key_of(&site.create(&mut ada, "Work of Bob").await?)?;
        comment(&mut bob, &issue, "Bob's comment").await?;
        ok(&bob
            .send("POST", &format!("{MINE}/start"), Some(json!({})))
            .await?)?;
        let (shared, group) =
            bob_leaves_traces(&site, &rig, &mut ada, &mut bob, &issue, &bob_id.to_string()).await?;
        // Ada joins the group the shared search is shared with, so she sees it.
        let admin = site.org.admin.id;
        let joined = ada
            .send(
                "PUT",
                &format!("/api/v1/groups/{group}/members/{admin}"),
                None,
            )
            .await?;
        assert!(joined.status.is_success(), "{}", joined.body);
        // Only a switched-off person is removed.
        let active = remove(&mut ada, bob_id, json!({})).await?;
        assert_eq!(active["code"], "person.not-switched-off", "{active}");
        switch_off(&mut ada, bob_id).await?;
        // Only a holder of Remove people removes: an active team member is refused.
        let (mut cy, _) = site
            .person_called(StartingRole::TeamMember, "cy", "Cy Clerk")
            .await?;
        let refused = remove(&mut cy, bob_id, json!({})).await?;
        assert_eq!(refused["status"], 403, "{refused}");
        assert_eq!(refused["code"], "access.forbidden", "{refused}");
        let done = remove(&mut ada, bob_id, json!({"comments": "keep"})).await?;
        assert_eq!(done["status"], 204, "{done}");
        rig.run_jobs().await?;
        // His comment stays, shown as Former user.
        let shown = comments(&mut ada, &issue).await?;
        assert!(shown.contains("Bob's comment"), "{shown}");
        assert!(shown.contains("\"former\""), "{shown}");
        assert!(!shown.contains(NAME), "{shown}");
        // The shared search stays, with Former user as its maker.
        let search = ok(&ada
            .send("GET", &format!("/api/v1/saved-searches/{shared}"), None)
            .await?)?
        .to_string();
        assert!(search.contains("\"former\""), "{search}");
        assert!(!search.contains(NAME), "{search}");
        // The walk finds nothing that only served his account.
        assert_eq!(left_to_delete(&rig, bob_id).await?, Vec::<String>::new());
        // No column of any table holds his name, handle or address but the legal-hold field; a
        // name outside ASCII is swept for in `removal_others_http`.
        assert_eq!(
            rig.storage
                .columns_holding(&[NAME, HANDLE, ADDRESS])
                .await?,
            vec!["person_standing.hold_name".to_owned()]
        );
        // The changes keep neither his name nor his address.
        let events = events_text(&rig, bob_id).await?;
        assert!(!events.contains(NAME), "{events}");
        assert!(!events.contains(ADDRESS), "{events}");
        // The tombstone: a label, the name only in the legal-hold field for his work day.
        let standing = rig
            .storage
            .standing(bob_id)
            .await?
            .ok_or("Bob has a standing")?;
        assert_eq!(standing.standing, Standing::Removed);
        assert!(standing.label.is_some());
        assert_eq!(standing.hold_name.as_deref(), Some(NAME));
        assert_eq!(standing.hold_reason, Some(HoldReason::WorkDays));
        assert!(standing.hold_until.is_some());
        let account = rig.account_by_email(ADDRESS).await?;
        assert!(account.is_none(), "the address is cleared");
        // No undo: he is not removed twice, and not switched on.
        let again = remove(&mut ada, bob_id, json!({})).await?;
        assert_eq!(again["code"], "resource.not-found", "{again}");
        let on = ada
            .send("POST", &format!("/api/v1/people/{bob_id}/switch-on"), None)
            .await?;
        assert_eq!(on.code(), "person.not-switched-off", "{}", on.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_delete_their_comments_deletes_comments_and_nothing_else() -> TestResult {
    each_database("priv_04_removal_comments_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        let issue = key_of(&site.create(&mut ada, "Talk about it").await?)?;
        comment(&mut bob, &issue, "Bob's words").await?;
        comment(&mut ada, &issue, "Ada's words").await?;
        rig.run_jobs().await?;
        switch_off(&mut ada, bob_id).await?;
        let done = remove(&mut ada, bob_id, json!({"comments": "delete"})).await?;
        assert_eq!(done["status"], 204, "{done}");
        let shown = comments(&mut ada, &issue).await?;
        assert!(!shown.contains("Bob's words"), "{shown}");
        assert!(shown.contains("Ada's words"), "{shown}");
        // The issue he did not write stays.
        let found = ada
            .send("GET", &format!("/api/v1/issues/{issue}"), None)
            .await?;
        ok(&found)?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_the_events_keep_no_value_of_the_persons_settings_or_private_searches() -> TestResult
{
    each_database("priv_04_removal_scrub_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        ok(&bob
            .send(
                "PUT",
                "/api/v1/me/settings/appearance.mode",
                Some(json!({"value": "dark"})),
            )
            .await?)?;
        super::removal_plan_http::save_search(&mut bob, "Zephyr private list").await?;
        let before = events_text(&rig, bob_id).await?;
        assert!(before.contains("Zephyr private list"), "{before}");
        assert!(before.contains("dark"), "{before}");
        switch_off(&mut ada, bob_id).await?;
        let done = remove(&mut ada, bob_id, json!({})).await?;
        assert_eq!(done["status"], 204, "{done}");
        let after = events_text(&rig, bob_id).await?;
        assert!(!after.contains("Zephyr private list"), "{after}");
        assert!(!after.contains("dark"), "{after}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_work_times_go_in_the_change_that_removes_the_person() -> TestResult {
    each_database("priv_04_removal_work_times_http", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::TeamMember, "bob").await?;
        ok(&bob
            .send(
                "POST",
                "/api/v1/me/away-times",
                Some(json!({"first": "2027-03-23", "last": "2027-03-24"})),
            )
            .await?)?;
        assert!(
            left_to_delete(&rig, bob_id)
                .await?
                .iter()
                .any(|kind| kind.contains("away")),
            "the away time is found before"
        );
        switch_off(&mut ada, bob_id).await?;
        // An empty body takes the setting's choice for the comments.
        let done = ada
            .send("POST", &format!("/api/v1/people/{bob_id}/remove"), None)
            .await?;
        assert_eq!(done.status.as_u16(), 204, "{}", done.body);
        // The walk finds nothing of the account left, the away time included: it went with the
        // change that removed him.
        assert_eq!(left_to_delete(&rig, bob_id).await?, Vec::<String>::new());
        Ok(())
    })
    .await
}
