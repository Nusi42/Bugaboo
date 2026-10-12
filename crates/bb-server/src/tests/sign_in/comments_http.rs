//! Comments, mentions, history and following over HTTP (ISS-05, ISS-06, ISS-07, SEC-06): comments
//! kept with every earlier text, mentions that follow *Who sees people*, issue IDs that are chips
//! only for readers who may see the issue, a history that leaves out what the reader may not see,
//! and followers. Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_app::comments::CommentStore as _;
use bb_app::history::{HistoryStore as _, catch_up, catch_up_batch};
use bb_app::roles::SetFieldLevel;
use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_core::history::HistoryKind;
use bb_core::id::{CommentId, IssueId};
use bb_core::text::LONG_TEXT_MAX;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::http::{Browser, Reply, rig_and_app};
use super::issues_site::{ISSUES, Site, ok};
use super::policy_rig::Member;
use super::rig::Rig;

/// An editor id for the lock requests of these tests.
const AN_EDITOR: &str = "7b0c1e2a-3d4f-4a5b-8c6d-7e8f9a0b1c2d";

fn key_of(reply: &Reply) -> Result<String, Failure> {
    ok(reply)?["key"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "the answer holds no issue ID".into())
}

fn id_of(reply: &Reply) -> Result<String, Failure> {
    ok(reply)?["id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "the answer holds no ID".into())
}

/// Writes a comment; the answer.
async fn comment(browser: &mut Browser, issue: &str, text: &str) -> Result<Reply, Failure> {
    browser
        .send(
            "POST",
            &format!("{ISSUES}/{issue}/comments"),
            Some(json!({"text": text})),
        )
        .await
}

/// Takes the lock on the text of a comment; its token.
async fn lock(browser: &mut Browser, comment: &str) -> Result<String, Failure> {
    let taken = browser
        .send(
            "POST",
            "/api/v1/edit-locks",
            Some(json!({
                "what": {"record": {"kind": "comment", "id": comment}, "part": "text"},
                "editor_id": AN_EDITOR,
            })),
        )
        .await?;
    Ok(ok(&taken)?["lock_token"]
        .as_str()
        .ok_or("no lock token")?
        .to_owned())
}

/// The items of a list.
fn items(list: &Reply) -> Vec<Value> {
    list.body["items"].as_array().cloned().unwrap_or_default()
}

#[tokio::test]
async fn iss_05_an_edited_comment_keeps_its_earlier_versions() -> TestResult {
    each_database("iss_05_versions", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.make(&mut ada, "Printer jams").await?)?;
        let written = comment(&mut ada, &issue, "It jams on page two.").await?;
        let id = id_of(&written)?;
        assert_eq!(written.body["version"], 1);
        assert!(
            written.body["html"]
                .as_str()
                .is_some_and(|html| html.contains("<p>"))
        );
        // The text is edited under the lock, on the version the editor started from.
        let token = lock(&mut ada, &id).await?;
        let saved = ada
            .send(
                "PATCH",
                &format!("/api/v1/comments/{id}"),
                Some(json!({"version": 1, "lock_token": token, "text": "It jams on page three."})),
            )
            .await?;
        assert_eq!(ok(&saved)?["version"], 2);
        assert_eq!(saved.body["text"], "It jams on page three.");
        assert!(saved.body["edited_at"].is_string());
        // Every text is kept, the current one last.
        let versions = ada
            .send("GET", &format!("/api/v1/comments/{id}/versions"), None)
            .await?;
        let texts: Vec<String> = items(&versions)
            .iter()
            .map(|item| item["text"].as_str().unwrap_or_default().to_owned())
            .collect();
        assert_eq!(texts, ["It jams on page two.", "It jams on page three."]);
        // The lock is gone, so a second save with the old token and version is refused, and a
        // stale version is told as a conflict.
        let again = ada
            .send(
                "PATCH",
                &format!("/api/v1/comments/{id}"),
                Some(json!({"version": 1, "lock_token": token, "text": "Late."})),
            )
            .await?;
        assert_eq!(again.status, StatusCode::CONFLICT);
        let fresh = lock(&mut ada, &id).await?;
        let stale = ada
            .send(
                "PATCH",
                &format!("/api/v1/comments/{id}"),
                Some(json!({"version": 1, "lock_token": fresh, "text": "Stale."})),
            )
            .await?;
        assert_eq!(stale.code(), "edit.version_changed");
        assert_eq!(stale.body["conflict"]["version"], 2);
        // Only the author changes a text: another person cannot even take the lock.
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let refused = bob
            .send(
                "POST",
                "/api/v1/edit-locks",
                Some(json!({
                    "what": {"record": {"kind": "comment", "id": id}, "part": "text"},
                    "editor_id": AN_EDITOR,
                })),
            )
            .await?;
        assert_eq!(refused.code(), "resource.not_found");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_05_a_comment_goes_to_the_trash_and_comes_back_and_needs_the_permission() -> TestResult
{
    each_database("iss_05_trash", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.make(&mut ada, "Printer jams").await?)?;
        // A Viewer may see the issue, not comment on it.
        let (mut vera, _) = site.person(StartingRole::Viewer, "vera").await?;
        let refused = comment(&mut vera, &issue, "Me too.").await?;
        assert_eq!(
            (refused.status, refused.code()),
            (StatusCode::FORBIDDEN, "access.forbidden")
        );
        let empty = comment(&mut ada, &issue, "   ").await?;
        assert_eq!(empty.code(), "request.invalid");
        let id = id_of(&comment(&mut ada, &issue, "First.").await?)?;
        let trashed = ada
            .send("DELETE", &format!("/api/v1/comments/{id}"), None)
            .await?;
        assert_eq!(trashed.status, StatusCode::NO_CONTENT);
        let gone = ada
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?tab=comments"),
                None,
            )
            .await?;
        assert!(items(&gone).is_empty());
        let back = ada
            .send("POST", &format!("/api/v1/comments/{id}/restore"), None)
            .await?;
        assert_eq!(ok(&back)?["id"], id);
        let list = ada
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?tab=comments"),
                None,
            )
            .await?;
        assert_eq!(items(&list).len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_05_a_guest_can_mention_a_role_but_not_a_person() -> TestResult {
    each_database("iss_05_guest_mentions", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = site
            .person_called(StartingRole::Guest, "gus", "Gus Guest")
            .await?;
        let (mut lena, _) = site
            .person_called(StartingRole::ProjectLead, "lena", "Lena Lead")
            .await?;
        let issue = key_of(&site.make(&mut gus, "Gus cannot sign in").await?)?;
        // What Gus may name: groups and roles, no person.
        let choices = gus
            .send("GET", &format!("{ISSUES}/{issue}/mention-choices"), None)
            .await?;
        let targets: Vec<&str> = ok(&choices)?["items"]
            .as_array()
            .ok_or("no items")?
            .iter()
            .filter_map(|item| item["target"].as_str())
            .collect();
        assert!(targets.contains(&"role"), "{targets:?}");
        assert!(!targets.contains(&"person"), "{targets:?}");
        // Ada, who sees names, may name people (Lena) and groups, not roles.
        let ada_choices = ada
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/mention-choices?search=lena"),
                None,
            )
            .await?;
        let found = items(&ada_choices);
        assert!(
            found
                .iter()
                .any(|item| item["target"] == "person" && item["handle"] == "lena.lead"),
            "{found:?}"
        );
        assert!(
            found.iter().all(|item| item["target"] != "role"),
            "{found:?}"
        );
        // Gus mentions the role and, by name, Ada: only the role is a chip.
        let written = comment(
            &mut gus,
            &issue,
            "Help, @project.lead, and also @ada.admin please.",
        )
        .await?;
        let html = ok(&written)?["html"].as_str().ok_or("no html")?.to_owned();
        assert_eq!(html.matches("data-kind=\"mention\"").count(), 1, "{html}");
        assert!(html.contains("@ada.admin"), "{html}");
        // The role's holder finds the comment under Mentions, unread; Ada does not.
        let lenas = lena.send("GET", "/api/v1/me/mentions", None).await?;
        let mine = items(&lenas);
        assert_eq!(mine.len(), 1, "{:?}", lenas.body);
        assert_eq!(mine[0]["unread"], true);
        assert_eq!(mine[0]["issue"]["key"], issue);
        let adas = ada.send("GET", "/api/v1/me/mentions", None).await?;
        assert!(items(&adas).is_empty());
        // Reading it marks it, and it moves behind the unread ones.
        let comment_id = mine[0]["comment"].as_str().ok_or("no comment")?.to_owned();
        let marked = lena
            .send(
                "POST",
                "/api/v1/me/mentions/read",
                Some(json!({"comments": [comment_id]})),
            )
            .await?;
        assert_eq!(marked.status, StatusCode::NO_CONTENT);
        let after = lena.send("GET", "/api/v1/me/mentions", None).await?;
        assert_eq!(items(&after)[0]["unread"], false);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_05_a_hidden_issue_id_in_a_comment_stays_text() -> TestResult {
    each_database("iss_05_hidden_id", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let theirs = key_of(&site.make(&mut gus, "Gus's problem").await?)?;
        let secret = key_of(&site.make(&mut ada, "Ada's secret").await?)?;
        let own = key_of(&site.make(&mut gus, "Another problem of Gus").await?)?;
        let written = comment(&mut gus, &theirs, &format!("See {secret} and {own}.")).await?;
        let html = ok(&written)?["html"].as_str().ok_or("no html")?.to_owned();
        // Gus sees his own issue as a chip and the other one as text.
        assert_eq!(html.matches("data-kind=\"issue-id\"").count(), 1, "{html}");
        assert!(html.contains(&secret) && html.contains(&own), "{html}");
        assert!(!html.contains(&format!(">{secret}</span>")), "{html}");
        // Ada, who sees both, sees two chips in the same comment.
        let history = ada
            .send(
                "GET",
                &format!("{ISSUES}/{theirs}/history?tab=comments"),
                None,
            )
            .await?;
        let shown = items(&history);
        let html = shown[0]["html"].as_str().ok_or("no html")?;
        assert_eq!(html.matches("data-kind=\"issue-id\"").count(), 2, "{html}");
        Ok(())
    })
    .await
}

/// A reader who sees an issue's title and status but not its priority.
async fn reader_without_priority(site: &Site<'_>, rig: &Rig) -> Result<Browser, Failure> {
    let role = site
        .org
        .role("Reader", &[Permission::SeeProjects, Permission::SeeIssues])
        .await?;
    for field in [Field::Title, Field::Status] {
        rig.api()
            .set_field_level(
                Rig::info(),
                SetFieldLevel {
                    session_token: site.org.admin.session.clone(),
                    role: role.id,
                    project: None,
                    issue_type: None,
                    field: FieldKey::BuiltIn(field),
                    level: Some(FieldLevel::CanSee),
                },
            )
            .await?;
    }
    let email = "reader@example.test";
    let started = rig.person(&site.org.admin.session, email).await?;
    let member = Member {
        id: started.person.id,
        session: started.session_token,
    };
    site.org.give_everywhere(&member, role.id).await?;
    Browser::signed_in_as(site.app, email).await
}

#[tokio::test]
async fn iss_06_history_leaves_out_changes_to_fields_the_reader_cannot_see() -> TestResult {
    each_database("iss_06_mask", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut reader = reader_without_priority(&site, &rig).await?;
        let issue = key_of(&site.make(&mut ada, "Printer jams").await?)?;
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": 1, "priority": "critical", "status": "in-progress"})),
            )
            .await?)?;
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": 2, "priority": "low"})),
            )
            .await?)?;
        let fields = |history: &Reply| -> Vec<String> {
            items(history)
                .iter()
                .filter_map(|item| item["field"].as_str().map(str::to_owned))
                .collect()
        };
        // Ada sees every change, the reader the ones to fields they may see; a change that is
        // only to a field they may not see is not there at all, not even as a gap.
        let all = ada
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?tab=changes"),
                None,
            )
            .await?;
        let all = fields(&all);
        assert_eq!(
            all.iter().filter(|field| *field == "priority").count(),
            3,
            "made, then changed twice: {all:?}"
        );
        let seen = reader
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?tab=changes"),
                None,
            )
            .await?;
        let seen_fields = fields(&seen);
        assert!(
            !seen_fields.iter().any(|field| field == "priority"),
            "{seen_fields:?}"
        );
        assert!(
            seen_fields.iter().any(|field| field == "status"),
            "{seen_fields:?}"
        );
        // The reader may not ask for what they may not see either.
        let asked = reader
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?fields=event,priority"),
                None,
            )
            .await?;
        assert_eq!(asked.code(), "request.unknown_field");
        // Their view of the author is as their level for the kind says, not Ada's name.
        let by = &seen.body["items"][0]["by"];
        assert_eq!(by["shown"], "someone", "{by}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_06_the_history_reader_resumes_after_a_restart() -> TestResult {
    each_database("iss_06_resume", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.make(&mut ada, "Printer jams").await?)?;
        for text in ["One.", "Two.", "Three."] {
            ok(&comment(&mut ada, &issue, text).await?)?;
        }
        // The consumer reads a little, and the process stops.
        let now = rig.clock_now();
        let first = catch_up_batch(&rig.storage, now, 2).await?;
        assert_eq!(first, 2);
        // A new process: another storage on the same database resumes from the place recorded.
        let again = rig.storage.clone();
        let rest = catch_up(&again, now).await?;
        assert!(rest > 0);
        let nothing = catch_up(&again, now).await?;
        assert_eq!(nothing, 0, "everything was read once");
        // Each event made its row once: the issue and three comments.
        let issue_id: IssueId =
            ok(&ada.send("GET", &format!("{ISSUES}/{issue}"), None).await?)?["issue"]["id"]
                .as_str()
                .ok_or("no ID")?
                .parse()?;
        let rows = again.history_page(issue_id, None, None, 50).await?;
        assert_eq!(rows.len(), 4, "{rows:?}");
        assert_eq!(
            rows.iter()
                .filter(|row| row.kind == HistoryKind::Comment)
                .count(),
            3
        );
        // Reading the page catches up again and adds nothing twice.
        let page = ada
            .send("GET", &format!("{ISSUES}/{issue}/history"), None)
            .await?;
        let comments = items(&page)
            .iter()
            .filter(|item| item["kind"] == "comment")
            .count();
        assert_eq!(comments, 3);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_07_reporting_assigning_and_commenting_start_following() -> TestResult {
    each_database("iss_07_following", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, _) = site.person(StartingRole::TeamMember, "bob").await?;
        let (mut cora, cora_id) = site.person(StartingRole::TeamMember, "cora").await?;
        let issue = key_of(&site.make(&mut ada, "Printer jams").await?)?;
        let names = |list: &Reply| -> Vec<String> {
            items(list)
                .iter()
                .filter_map(|item| item["name"].as_str().map(str::to_owned))
                .collect()
        };
        // The person who reported it follows it.
        let followers = ada
            .send("GET", &format!("{ISSUES}/{issue}/followers"), None)
            .await?;
        assert_eq!(names(&followers), ["Ada Admin"]);
        // Whoever it is assigned to follows it.
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": 1, "assigned_to": [cora_id.to_string()]})),
            )
            .await?)?;
        // Whoever comments follows it.
        ok(&comment(&mut bob, &issue, "I can reproduce it.").await?)?;
        let followers = ada
            .send("GET", &format!("{ISSUES}/{issue}/followers"), None)
            .await?;
        let mut shown = names(&followers);
        shown.sort();
        assert!(shown.contains(&"Ada Admin".to_owned()), "{shown:?}");
        assert_eq!(shown.len(), 3);
        // Following is the person's own choice: Cora leaves, and Bob is told he follows.
        let state = bob
            .send("GET", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert_eq!(ok(&state)?["following"], true);
        assert_eq!(state.body["reason"], "commented");
        let left = cora
            .send("DELETE", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert_eq!(left.status, StatusCode::NO_CONTENT);
        let gone = cora
            .send("GET", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert_eq!(ok(&gone)?["following"], false);
        let joined = cora
            .send("PUT", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert_eq!(joined.status, StatusCode::NO_CONTENT);
        let back = cora
            .send("GET", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert_eq!(back.body["reason"], "chose");
        let all = ada
            .send("GET", &format!("{ISSUES}/{issue}/followers"), None)
            .await?;
        assert_eq!(items(&all).len(), 3);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_07_a_followers_list_hidden_for_the_reader_is_left_out_with_its_length() -> TestResult {
    each_database("iss_07_followers_hidden", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, _) = site.person(StartingRole::Guest, "gus").await?;
        let issue = key_of(&site.make(&mut gus, "Gus cannot print").await?)?;
        // Ada comments, so two people follow the issue.
        ok(&comment(&mut ada, &issue, "On it.").await?)?;
        let for_ada = ada
            .send("GET", &format!("{ISSUES}/{issue}/followers"), None)
            .await?;
        assert_eq!(items(&for_ada).len(), 2);
        // A guest sees no names of followers, and not how many there are either.
        let for_gus = gus
            .send("GET", &format!("{ISSUES}/{issue}/followers"), None)
            .await?;
        assert!(items(&for_gus).is_empty(), "{:?}", for_gus.body);
        assert!(for_gus.body["next_cursor"].is_null(), "{:?}", for_gus.body);
        // Their own choice is still theirs.
        let own = gus
            .send("GET", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert_eq!(ok(&own)?["following"], true);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_07_a_viewer_sees_the_followers_by_role_and_not_by_name() -> TestResult {
    each_database("iss_07_followers_role_only", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut bob, bob_id) = site.person(StartingRole::Viewer, "bob").await?;
        let issue = key_of(&site.make(&mut ada, "Printer jams").await?)?;
        let (mut cora, cora_id) = site
            .person_called(StartingRole::TeamMember, "cora", "Cora Carpenter")
            .await?;
        ok(&comment(&mut cora, &issue, "On it.").await?)?;
        let for_ada = ada
            .send("GET", &format!("{ISSUES}/{issue}/followers"), None)
            .await?;
        assert_eq!(items(&for_ada).len(), 2);
        // A viewer is told how many follow and what role each has, but no name and no ID.
        let for_bob = bob
            .send("GET", &format!("{ISSUES}/{issue}/followers"), None)
            .await?;
        let shown = items(&for_bob);
        assert_eq!(shown.len(), 2, "{:?}", for_bob.body);
        assert!(
            shown.iter().all(|item| item["shown"] == "role"),
            "{shown:?}"
        );
        let text = for_bob.body.to_string();
        for secret in ["Ada Admin", "Cora Carpenter", &cora_id.to_string()] {
            assert!(!text.contains(secret), "{secret} in {text}");
        }
        // Bob follows nothing here, and the list does not pretend he does.
        assert!(!text.contains(&bob_id.to_string()));
        let own = bob
            .send("GET", &format!("{ISSUES}/{issue}/follow"), None)
            .await?;
        assert_eq!(ok(&own)?["following"], false);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_05_a_comment_that_is_too_long_is_refused_and_nothing_is_stored() -> TestResult {
    each_database("iss_05_comment_too_long", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let issue = key_of(&site.make(&mut ada, "Long story").await?)?;
        let fits = comment(&mut ada, &issue, &"a".repeat(LONG_TEXT_MAX)).await?;
        ok(&fits)?;
        let too_long = comment(&mut ada, &issue, &"a".repeat(LONG_TEXT_MAX + 1)).await?;
        assert_eq!(
            (too_long.status, too_long.code()),
            (StatusCode::BAD_REQUEST, "request.invalid"),
            "{:?}",
            too_long.body
        );
        // Only the one that fits is under the issue.
        let page = ada
            .send("GET", &format!("{ISSUES}/{issue}/history"), None)
            .await?;
        let comments = items(&page)
            .iter()
            .filter(|item| item["kind"] == "comment")
            .count();
        assert_eq!(comments, 1, "{:?}", page.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_05_a_mention_does_not_reach_a_person_who_cannot_see_the_issue() -> TestResult {
    each_database("iss_05_mention_unseen", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut gus, gus_id) = site
            .person_called(StartingRole::Guest, "gus", "Gus Guest")
            .await?;
        let (_, bob_id) = site
            .person_called(StartingRole::TeamMember, "bob", "Bob Builder")
            .await?;
        // Gus reported nothing here, so he cannot see Ada's issue; Ada names him and Bob.
        let issue = key_of(&site.make(&mut ada, "Budget plan").await?)?;
        let written = comment(&mut ada, &issue, "Please ask @gus.guest and @bob.builder.").await?;
        let comment_id: CommentId = ok(&written)?["id"].as_str().ok_or("no id")?.parse()?;
        // The rows that are stored: Bob has one, Gus has none.
        let rows = rig.storage.mentions_in_comment(comment_id).await?;
        let people: Vec<_> = rows.iter().map(|row| row.recipient).collect();
        assert_eq!(people, [bob_id], "{rows:?}");
        assert!(!people.contains(&gus_id));
        let theirs = gus.send("GET", "/api/v1/me/mentions", None).await?;
        assert!(items(&theirs).is_empty(), "{:?}", theirs.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn iss_06_a_page_of_hidden_rows_still_answers_with_a_cursor_and_repeats_nothing() -> TestResult
{
    each_database("iss_06_bounded", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut reader = reader_without_priority(&site, &rig).await?;
        let issue = key_of(&site.make(&mut ada, "Printer jams").await?)?;
        // One change the reader sees, then many they do not (the newest are read first).
        ok(&ada
            .send(
                "PATCH",
                &format!("{ISSUES}/{issue}"),
                Some(json!({"version": 1, "status": "in-progress"})),
            )
            .await?)?;
        for version in 2..=30 {
            let priority = if version % 2 == 0 { "low" } else { "high" };
            ok(&ada
                .send(
                    "PATCH",
                    &format!("{ISSUES}/{issue}"),
                    Some(json!({"version": version, "priority": priority})),
                )
                .await?)?;
        }
        let first = reader
            .send(
                "GET",
                &format!("{ISSUES}/{issue}/history?tab=changes&limit=1"),
                None,
            )
            .await?;
        // Nothing to show on the first page, but the history goes on: a cursor comes back.
        assert!(items(&first).is_empty(), "{:?}", first.body);
        let mut cursor = ok(&first)?["next_cursor"]
            .as_str()
            .ok_or("no cursor after a page of hidden rows")?
            .to_owned();
        let mut events: Vec<String> = Vec::new();
        for _ in 0..40 {
            let page = reader
                .send(
                    "GET",
                    &format!("{ISSUES}/{issue}/history?tab=changes&limit=1&cursor={cursor}"),
                    None,
                )
                .await?;
            events.extend(items(&page).iter().filter_map(|item| {
                // An event with several fields has one item for each of them.
                Some(format!(
                    "{}/{}",
                    item["event"].as_str()?,
                    item["field"].as_str()?
                ))
            }));
            match ok(&page)?["next_cursor"].as_str() {
                Some(next) => next.clone_into(&mut cursor),
                None => break,
            }
        }
        let mut unique = events.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), events.len(), "no change twice: {events:?}");
        assert!(!events.is_empty(), "the change the reader sees is reached");
        Ok(())
    })
    .await
}
