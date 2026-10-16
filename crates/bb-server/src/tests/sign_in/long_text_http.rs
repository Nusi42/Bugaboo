//! What every long text of an issue promises (ADR-0019, ISS-17), once for the description and once
//! for the long text of an own field: a change of another field does not refuse a text saved under
//! its lock, and a text that was saved since refuses the stale save with the structured answer and
//! keeps what the person wrote as their draft. The two are the routes of `LONG_TEXT_ROUTES` that
//! belong to an issue (the third, a comment, has its own record in `comments_http`).

use axum::http::StatusCode;
use bb_core::edit::DESCRIPTION_PART;
use bb_core::own_fields::EDIT_PART_PREFIX;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::{Value, json};

use super::fields_http::create_field;
use super::http::{Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, id_in, key_in, ok};

const LOCKS: &str = "/api/v1/edit-locks";
const DRAFTS: &str = "/api/v1/drafts";
const AN_EDITOR: &str = "7c1d2e3f-4a5b-4c6d-8e7f-a0b1c2d3e4f5";

/// A long text of an issue.
#[derive(Clone, Copy, Debug)]
enum LongText {
    /// The description.
    Description,
    /// The long text of an own field.
    OwnField,
}

const LONG_TEXTS: [LongText; 2] = [LongText::Description, LongText::OwnField];

/// An issue with one of its long texts: where it is saved and what its lock is about.
struct Target {
    issue: String,
    key: String,
    /// The part of the record the lock and the draft are about.
    part: String,
    /// The route that saves the text.
    route: String,
    /// What a change of the text is called in the conflict answer.
    member: String,
}

impl Target {
    async fn create(text: LongText, site: &Site<'_>, ada: &mut Browser) -> Result<Self, Failure> {
        let made = ok(&site.create(ada, "Has a long text").await?)?.clone();
        let (issue, key) = (id_in(&made)?, key_in(&made)?);
        Ok(match text {
            LongText::Description => Self {
                route: format!("{ISSUES}/{key}/description"),
                part: DESCRIPTION_PART.to_owned(),
                member: DESCRIPTION_PART.to_owned(),
                issue,
                key,
            },
            LongText::OwnField => {
                let notes = create_field(
                    ada,
                    json!({"name": "Notes", "kind": "long-text", "applies_to": "issues"}),
                )
                .await?;
                let field = id_in(&notes)?;
                Self {
                    route: format!("{ISSUES}/{key}/fields/{field}/text"),
                    part: format!("{EDIT_PART_PREFIX}{field}"),
                    member: field,
                    issue,
                    key,
                }
            }
        })
    }

    /// Takes the lock on the text; its token.
    async fn lock(&self, ada: &mut Browser) -> Result<String, Failure> {
        let taken = ada
            .send(
                "POST",
                LOCKS,
                Some(json!({
                    "what": {"record": {"kind": "issue", "id": self.issue}, "part": self.part},
                    "editor_id": AN_EDITOR,
                })),
            )
            .await?;
        assert_eq!(taken.body["status"], "taken", "{}", taken.body);
        Ok(ok(&taken)?["lock_token"]
            .as_str()
            .ok_or("no lock token")?
            .to_owned())
    }

    /// Saves the text under a new lock on this version.
    async fn save(
        &self,
        ada: &mut Browser,
        version: i64,
        text: &str,
    ) -> Result<crate::sign_in_tests::http::Reply, Failure> {
        let token = self.lock(ada).await?;
        ada.send(
            "PUT",
            &self.route,
            Some(json!({"version": version, "lock_token": token, "text": text})),
        )
        .await
    }
}

#[tokio::test]
async fn adr_0019_a_change_of_another_field_does_not_refuse_a_long_text() -> TestResult {
    each_database("adr_0019_long_text_other_field", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for text in LONG_TEXTS {
            let target = Target::create(text, &site, &mut ada).await?;
            let token = target.lock(&mut ada).await?;
            // Somebody changes the priority meanwhile: the version of the issue counts up, and the
            // text is not what they changed.
            let other = ada
                .send(
                    "PATCH",
                    &format!("{ISSUES}/{}", target.key),
                    Some(json!({"version": 1, "priority": "high"})),
                )
                .await?;
            assert_eq!(other.status, StatusCode::OK, "{text:?}: {}", other.body);
            let saved = ada
                .send(
                    "PUT",
                    &target.route,
                    Some(json!({"version": 1, "lock_token": token, "text": "My text"})),
                )
                .await?;
            assert_eq!(saved.status, StatusCode::OK, "{text:?}: {}", saved.body);
            assert_eq!(saved.body["version"], 3, "{text:?}");
            assert_eq!(saved.body["priority"], "high", "{text:?}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0019_a_stale_long_text_gets_the_structured_answer_and_the_text_is_kept() -> TestResult
{
    each_database("adr_0019_long_text_stale", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        for text in LONG_TEXTS {
            let target = Target::create(text, &site, &mut ada).await?;
            // The text itself changes meanwhile (the first save lets the lock go).
            let first = target.save(&mut ada, 1, "First").await?;
            assert_eq!(first.status, StatusCode::OK, "{text:?}: {}", first.body);
            let stale = target.save(&mut ada, 1, "My text").await?;
            assert_eq!(
                stale.status,
                StatusCode::CONFLICT,
                "{text:?}: {}",
                stale.body
            );
            assert_eq!(stale.code(), "edit.version_changed", "{text:?}");
            let conflict = &stale.body["conflict"];
            assert_eq!(conflict["version"], 2, "{text:?}: {}", stale.body);
            assert_eq!(conflict["fields"], json!([target.member]), "{text:?}");
            // What the person wrote is kept as their draft.
            let draft = ada
                .send(
                    "GET",
                    &format!(
                        "{DRAFTS}?kind=issue&id={}&part={}",
                        target.issue, target.part
                    ),
                    None,
                )
                .await?;
            assert_eq!(
                ok(&draft)?["draft"]["text"],
                Value::from("My text"),
                "{text:?}"
            );
        }
        Ok(())
    })
    .await
}
