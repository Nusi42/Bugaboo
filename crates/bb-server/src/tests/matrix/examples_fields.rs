//! The example requests of the routes of own fields (M3 WP-05).
//!
//! Every example that changes a field makes a new one first, with two choices, so each actor who
//! is let in works on a field of their own and nobody else's work gets in the way. The routes that
//! read one field name a field that is not there: a person who may see no field is told so alike
//! for a field that exists and one that does not (CF-06).

use bb_app::fields::{CreateField, NewChoice};
use bb_core::own_fields::{FieldFor, FieldType, Rules, Scope};
use bb_testing::Failure;
use serde_json::json;

use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// An ID of the right kind of a field that nobody made.
const NO_SUCH_FIELD: &str = "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c";

/// A field of this type, made by the administrator of the matrix, with these choices: its ID and
/// its choices.
async fn a_field_of(
    fixture: &Fixture<'_>,
    kind: FieldType,
    choices: &[&str],
) -> Result<(String, Vec<String>), Failure> {
    let number = fixture.next_number();
    let made = fixture
        .rig()
        .api()
        .create_field(
            Rig::info(),
            CreateField {
                session_token: fixture.admin_session().to_owned(),
                name: format!("Severity {number}"),
                help: None,
                kind,
                applies_to: FieldFor::Issues,
                rules: Rules::default(),
                default_value: None,
                look: None,
                never_to_assistant: false,
                scope: Scope::default(),
                choices: choices
                    .iter()
                    .map(|name| NewChoice {
                        name: (*name).to_owned(),
                        color: None,
                        icon: None,
                    })
                    .collect(),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    Ok((
        made.id.to_string(),
        made.choices
            .iter()
            .map(|choice| choice.id.to_string())
            .collect(),
    ))
}

/// A field with two choices.
async fn a_new_field(fixture: &Fixture<'_>) -> Result<(String, Vec<String>), Failure> {
    a_field_of(fixture, FieldType::PickOne, &["Minor", "Major"]).await
}

pub(super) async fn field_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    Ok(Some(match operation {
        "listFieldTypes" => ("GET", "/api/v1/field-types".into(), None),
        "listFields" => ("GET", "/api/v1/fields".into(), None),
        "getField" => ("GET", format!("/api/v1/fields/{NO_SUCH_FIELD}"), None),
        "getFieldUsedBy" => (
            "GET",
            format!("/api/v1/fields/{NO_SUCH_FIELD}/used-by"),
            None,
        ),
        "createField" => (
            "POST",
            "/api/v1/fields".into(),
            Some(json!({
                "name": format!("Customer {}", fixture.next_number()),
                "kind": "text",
                "applies_to": "issues",
            })),
        ),
        "changeField" => (
            "PATCH",
            format!("/api/v1/fields/{}", a_new_field(fixture).await?.0),
            Some(json!({"version": 1, "help": "How bad is it"})),
        ),
        "addChoice" => (
            "POST",
            format!("/api/v1/fields/{}/choices", a_new_field(fixture).await?.0),
            Some(json!({"version": 1, "name": "Critical"})),
        ),
        "changeChoice" => {
            let (field, choices) = a_new_field(fixture).await?;
            (
                "PATCH",
                format!("/api/v1/fields/{field}/choices/{}", choices[0]),
                Some(json!({"version": 1, "color": "cat3"})),
            )
        }
        "reorderChoices" => {
            let (field, choices) = a_new_field(fixture).await?;
            (
                "PUT",
                format!("/api/v1/fields/{field}/choices/order"),
                Some(json!({"version": 1, "order": [choices[1], choices[0]]})),
            )
        }
        "combineChoices" => {
            let (field, choices) = a_new_field(fixture).await?;
            (
                "POST",
                format!("/api/v1/fields/{field}/choices/{}/combine", choices[1]),
                Some(json!({"version": 1, "into": choices[0]})),
            )
        }
        "saveIssueFieldText" => {
            let (field, _) = a_field_of(fixture, FieldType::LongText, &[]).await?;
            (
                "PUT",
                format!(
                    "/api/v1/issues/{}/fields/{field}/text",
                    super::examples_issues::an_issue(fixture).await?
                ),
                Some(json!({
                    "version": 1,
                    "lock_token": super::examples_editing::NO_LOCK_TOKEN,
                    "text": "Steps",
                })),
            )
        }
        _ => return Ok(None),
    }))
}
