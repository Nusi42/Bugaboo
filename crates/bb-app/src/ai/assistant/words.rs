//! The fields of a proposal in the words of the history (AI-05): the status by its name, people as
//! the reader may see them, so a card says what the history will say and the browser words it with
//! the one wording of changes.

use bb_core::event::FieldChange;
use serde_json::Value;

use crate::access::Actor;
use crate::ai::tools::ProposedChange;
use crate::auth::{AuthStore, Ctx};
use crate::history::FieldChangeView;

impl<S: AuthStore> Ctx<'_, S> {
    /// The changes of a tool that works on an issue, as the history reads them for the actor. A
    /// tool that works on no issue (a new one) or an issue the actor cannot see gives none, and a
    /// field the actor may not see is left out, as in the history.
    pub(super) async fn words_of(
        &self,
        actor: &Actor,
        input: &Value,
        changes: &[ProposedChange],
    ) -> Vec<FieldChangeView> {
        let Some(address) = input.get("issue").and_then(Value::as_str) else {
            return Vec::new();
        };
        let Ok((issue, held)) = self.visible_issue(actor, address).await else {
            return Vec::new();
        };
        let Ok(mut reader) = self.change_reader().await else {
            return Vec::new();
        };
        let raw: Vec<FieldChange> = changes
            .iter()
            .map(|change| FieldChange {
                field: change.field.clone(),
                before: change.before.clone(),
                after: change.after.clone(),
            })
            .collect();
        self.changes_as_read(&mut reader, actor, &held, &issue, &raw)
            .await
            .unwrap_or_default()
    }
}
