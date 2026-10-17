//! The exit review of M3 (WP-12), locks and drafts (ISS-17, ISS-11, SEC-06): a lock or a draft is
//! only for a part a kind of record lists and the role lets the person use, a person keeps a
//! bounded number of drafts, and ending somebody's editing needs the right to change the record.
//! Every flow runs on SQLite and PostgreSQL.

use axum::http::StatusCode;
use bb_app::access::{Permission, StartingRole};
use bb_core::access::FieldLevel;
use bb_core::edit::DRAFTS_PER_AUTHOR_MAX;
use bb_core::fields::Field;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::exit_masks::{issue_of_type, person_with_levels};
use super::fields_http::create_field;
use super::http::{Browser, rig_and_app};
use super::issues_site::{Site, id_in, ok};

const LOCKS: &str = "/api/v1/edit-locks";
const DRAFTS: &str = "/api/v1/drafts";
const AN_EDITOR: &str = "7c1d2e3f-4a5b-4c6d-8e7f-a0b1c2d3e4f5";
const A_TOKEN: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

fn what(issue: &str, part: &str) -> Value {
    json!({"record": {"kind": "issue", "id": issue}, "part": part})
}

async fn take(
    person: &mut Browser,
    issue: &str,
    part: &str,
) -> Result<super::http::Reply, Failure> {
    person
        .send(
            "POST",
            LOCKS,
            Some(json!({"what": what(issue, part), "editor_id": AN_EDITOR})),
        )
        .await
}

async fn draft(
    person: &mut Browser,
    issue: &str,
    part: &str,
) -> Result<super::http::Reply, Failure> {
    person
        .send(
            "PUT",
            DRAFTS,
            Some(json!({
                "target": {"kind": "issue", "id": issue, "part": part},
                "draft": {"text": "typed", "base_version": 1},
            })),
        )
        .await
}

#[tokio::test]
async fn sec_06_a_lock_or_draft_is_only_for_a_part_the_record_has_and_the_role_lets_in()
-> TestResult {
    each_database("sec_06_valid_parts", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let notes = id_in(
            &create_field(
                &mut ada,
                json!({"name": "Notes", "kind": "long-text", "applies_to": "issues"}),
            )
            .await?,
        )?;
        let part = format!("field-{notes}");
        let (_, id) = issue_of_type(&site, &mut ada, &site.bug, "A bug").await?;
        // A part that does not exist is refused like a record that is not there.
        for nonsense in ["nonsense", "field-nonsense", "layout"] {
            let taken = take(&mut ada, &id, nonsense).await?;
            assert_eq!(taken.code(), "resource.not_found", "{nonsense}");
            let kept = draft(&mut ada, &id, nonsense).await?;
            assert_eq!(kept.code(), "resource.not_found", "{nonsense}");
        }
        // The two parts of an issue work.
        assert_eq!(
            ok(&take(&mut ada, &id, "description").await?)?["status"],
            "taken"
        );
        assert_eq!(ok(&take(&mut ada, &id, &part).await?)?["status"], "taken");
        // A role that changes issues but sees the long text only, and one that does not see it.
        let permissions = [
            Permission::SeeProjects,
            Permission::SeeIssues,
            Permission::ChangeIssues,
        ];
        let (mut sam, _) = person_with_levels(
            &site,
            &rig,
            "sam",
            &permissions,
            &[
                (Field::Title.into(), None, FieldLevel::CanChange),
                (Field::Description.into(), None, FieldLevel::CanChange),
                (
                    bb_core::fields::FieldKey::Own(notes.parse().map_err(|_| "no field")?),
                    None,
                    FieldLevel::CanSee,
                ),
            ],
        )
        .await?;
        let (mut eve, _) = person_with_levels(
            &site,
            &rig,
            "eve",
            &permissions,
            &[
                (Field::Title.into(), None, FieldLevel::CanChange),
                (Field::Description.into(), None, FieldLevel::CanChange),
            ],
        )
        .await?;
        let read = |part: &str| format!("{LOCKS}?kind=issue&id={id}&part={part}");
        // Sam sees the field: they may read who is editing it, not take it or draft on it.
        let held = sam.send("GET", &read(&part), None).await?;
        assert_eq!(held.status, StatusCode::OK, "{}", held.body);
        assert!(held.body["held"].is_object(), "{}", held.body);
        assert_eq!(take(&mut sam, &id, &part).await?.code(), "access.forbidden");
        assert_eq!(
            draft(&mut sam, &id, &part).await?.code(),
            "access.forbidden"
        );
        // Eve does not see it: no sign that somebody is editing, nothing to lock or draft.
        assert_eq!(
            eve.send("GET", &read(&part), None).await?.code(),
            "resource.not_found"
        );
        assert_eq!(
            take(&mut eve, &id, &part).await?.code(),
            "resource.not_found"
        );
        assert_eq!(
            draft(&mut eve, &id, &part).await?.code(),
            "resource.not_found"
        );
        // Both change the description.
        assert_eq!(
            draft(&mut eve, &id, "description").await?.status,
            StatusCode::OK
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_past_the_cap_the_oldest_draft_goes_and_no_save_is_refused() -> TestResult {
    each_database("sec_06_draft_cap", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let mut first = String::new();
        for number in 0..DRAFTS_PER_AUTHOR_MAX {
            let (_, id) =
                issue_of_type(&site, &mut ada, &site.bug, &format!("Issue {number}")).await?;
            let kept = draft(&mut ada, &id, "description").await?;
            assert_eq!(kept.status, StatusCode::OK, "{number}: {}", kept.body);
            rig.clock.advance(1);
            if number == 0 {
                first = id;
            }
        }
        // A new draft at the cap is kept, and the oldest one goes.
        let (_, one_more) = issue_of_type(&site, &mut ada, &site.bug, "One more").await?;
        let kept = draft(&mut ada, &one_more, "description").await?;
        assert_eq!(kept.status, StatusCode::OK, "{}", kept.body);
        let read = |id: &str| format!("{DRAFTS}?kind=issue&id={id}&part=description");
        assert!(ada.send("GET", &read(&first), None).await?.body["draft"].is_null());
        assert!(ada.send("GET", &read(&one_more), None).await?.body["draft"].is_object());
        // A save with a lock nobody holds is refused as before (`edit.lock_lost`), and the text is kept as
        // the draft, though the person is at the cap.
        rig.clock.advance(1);
        let (other, other_id) = issue_of_type(&site, &mut ada, &site.bug, "Another").await?;
        let lost = ada
            .send(
                "PUT",
                &format!("/api/v1/issues/{other}/description"),
                Some(json!({
                    "version": 1,
                    "lock_token": A_TOKEN,
                    "text": "Typed",
                })),
            )
            .await?;
        assert_eq!(lost.code(), "edit.lock_lost", "{}", lost.body);
        let kept = ada.send("GET", &read(&other_id), None).await?;
        assert_eq!(kept.body["draft"]["text"], "Typed", "{}", kept.body);
        // The heartbeat of a lock keeps renewing and keeping the text.
        let locked = ok(&take(&mut ada, &one_more, "description").await?)?.clone();
        let beat = ada
            .send(
                "POST",
                &format!("{LOCKS}/heartbeat"),
                Some(json!({
                    "what": what(&one_more, "description"),
                    "lock_token": locked["lock_token"],
                    "draft": {"text": "Still typing", "base_version": 1},
                })),
            )
            .await?;
        assert_eq!(beat.status, StatusCode::OK, "{}", beat.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_04_releasing_a_stuck_lock_needs_the_right_to_change_the_record_too() -> TestResult {
    each_database("sec_04_release_stuck", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (_, id) = issue_of_type(&site, &mut ada, &site.bug, "A bug").await?;
        let levels = [
            (Field::Title.into(), None, FieldLevel::CanChange),
            (Field::Description.into(), None, FieldLevel::CanChange),
        ];
        let (mut ray, _) = person_with_levels(
            &site,
            &rig,
            "ray",
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::EditAnyway,
            ],
            &levels,
        )
        .await?;
        let (mut kay, _) = person_with_levels(
            &site,
            &rig,
            "kay",
            &[
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::ChangeIssues,
                Permission::EditAnyway,
            ],
            &levels,
        )
        .await?;
        ok(&take(&mut ada, &id, "description").await?)?;
        let release = json!({"what": what(&id, "description")});
        let refused = ray
            .send(
                "POST",
                &format!("{LOCKS}/release-stuck"),
                Some(release.clone()),
            )
            .await?;
        assert_eq!(refused.code(), "access.forbidden", "{}", refused.body);
        // The lock is still Ada's.
        let held = ada
            .send(
                "GET",
                &format!("{LOCKS}?kind=issue&id={id}&part=description"),
                None,
            )
            .await?;
        assert!(held.body["held"].is_object(), "{}", held.body);
        let freed = kay
            .send("POST", &format!("{LOCKS}/release-stuck"), Some(release))
            .await?;
        assert_eq!(freed.status, StatusCode::NO_CONTENT, "{}", freed.body);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_layout_is_not_known_to_who_cannot_edit_layouts() -> TestResult {
    each_database("sec_06_layout_lock", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (mut tim, _) = site.person(StartingRole::TeamMember, "tim").await?;
        let editor = super::layouts_http::customize(&mut ada, "ACME").await?;
        let layout = editor["layout"]["id"].as_str().ok_or("no layout")?;
        let ask = |record: &str| {
            json!({
                "what": {"record": {"kind": "layout", "id": record}, "part": "layout"},
                "editor_id": AN_EDITOR,
            })
        };
        // A layout that exists and one that does not answer alike to a Team member.
        let there = tim.send("POST", LOCKS, Some(ask(layout))).await?;
        let nowhere = tim
            .send(
                "POST",
                LOCKS,
                Some(ask("01890a5d-ac96-774b-bcce-b302099a8057")),
            )
            .await?;
        assert_eq!(there.code(), "resource.not_found", "{}", there.body);
        assert_eq!(nowhere.code(), "resource.not_found", "{}", nowhere.body);
        // Whoever edits layouts takes the lock.
        let taken = ada.send("POST", LOCKS, Some(ask(layout))).await?;
        assert_eq!(ok(&taken)?["status"], "taken");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_comment_and_a_layout_have_one_part_each() -> TestResult {
    each_database("sec_06_other_kinds_parts", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (key, _) = issue_of_type(&site, &mut ada, &site.bug, "A bug").await?;
        let comment = ok(&ada
            .send(
                "POST",
                &format!("/api/v1/issues/{key}/comments"),
                Some(json!({"text": "First."})),
            )
            .await?)?["id"]
            .as_str()
            .ok_or("no comment")?
            .to_owned();
        let editor = super::layouts_http::customize(&mut ada, "ACME").await?;
        let layout = editor["layout"]["id"].as_str().ok_or("no layout")?;
        for (kind, id, good, bad) in [
            ("comment", comment.as_str(), "text", "description"),
            ("layout", layout, "layout", "text"),
        ] {
            let body = |part: &str| {
                json!({
                    "what": {"record": {"kind": kind, "id": id}, "part": part},
                    "editor_id": AN_EDITOR,
                })
            };
            let wrong = ada.send("POST", LOCKS, Some(body(bad))).await?;
            assert_eq!(wrong.code(), "resource.not_found", "{kind}: {}", wrong.body);
            let draft_bad = ada
                .send(
                    "PUT",
                    DRAFTS,
                    Some(json!({
                        "target": {"kind": kind, "id": id, "part": bad},
                        "draft": {"text": "typed", "base_version": 1},
                    })),
                )
                .await?;
            assert_eq!(draft_bad.code(), "resource.not_found", "{kind}");
            let right = ada.send("POST", LOCKS, Some(body(good))).await?;
            assert_eq!(ok(&right)?["status"], "taken", "{kind}");
        }
        Ok(())
    })
    .await
}
