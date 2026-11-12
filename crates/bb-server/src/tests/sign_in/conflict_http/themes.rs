//! The stale saves of an own theme: a personal theme of the person, saved under its edit lock or
//! taken away with the version in the query (ADR-0026, update M8).

use bb_testing::Failure;
use serde_json::{Value, json};

use super::{Save, made, text_in};
use crate::sign_in_tests::http::Browser;

/// Where own themes are made.
const THEMES: &str = "/api/v1/themes";

pub(super) async fn prepare(id: &str, ada: &mut Browser) -> Result<Save, Failure> {
    let theme = made(
        ada,
        THEMES,
        json!({"name": "Evening", "kind": "personal", "extends": "dusk"}),
    )
    .await?;
    let theme_id = text_in(&theme, "id")?;
    let path = format!("{THEMES}/{theme_id}");
    match id {
        "deleteTheme" => {
            Ok(Save::new("DELETE", path, Value::Null, &theme["version"])?.in_the_query())
        }
        "saveTheme" => Ok(Save::new(
            "PUT",
            path,
            json!({"name": "Evening", "values": {}}),
            &theme["version"],
        )?
        .under_lock(&json!({"kind": "theme", "id": theme_id}), "theme")),
        other => Err(format!("no row for {other}").into()),
    }
}
