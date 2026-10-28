//! The definitions of the own fields for a request: read once, and forgotten with every write of
//! a definition, so a read after a write in the same request sees the write.

use std::sync::Arc;

use bb_core::event::DomainEvent;

use super::types::Definition;
use crate::auth::{AppError, AuthStore, Ctx, internal, locked};
use crate::ports::{BoxFuture, ChangeStore};

impl<S: AuthStore> Ctx<'_, S> {
    /// The definitions of the own fields, read once for the request (every issue of a selection
    /// and every row of a page asks for them).
    pub(crate) async fn field_catalogue(&self) -> Result<Arc<Vec<Definition>>, AppError> {
        if let Some(known) = locked(&self.known.field_catalogue).clone() {
            return Ok(known);
        }
        let found = Arc::new(
            self.store()
                .field_definitions()
                .await
                .map_err(|error| internal::<S>(&error))?,
        );
        *locked(&self.known.field_catalogue) = Some(Arc::clone(&found));
        Ok(found)
    }

    /// Writes a change of a field, a scope or a choice with its event. The one way to write a
    /// definition: the definitions the request has read are dropped with the write (also when it
    /// fails), so no use case has to remember to.
    pub(crate) async fn record_change<'a, T: Send + 'a>(
        &'a self,
        event: &'a DomainEvent,
        change: impl for<'t> FnOnce(
            &'t mut <S as ChangeStore>::Transaction,
        ) -> BoxFuture<'t, Result<T, <S as ChangeStore>::Error>>
        + Send
        + 'a,
    ) -> Result<T, <S as ChangeStore>::Error> {
        let written = self.store().record_change(event, change).await;
        *locked(&self.known.field_catalogue) = None;
        written
    }
}
