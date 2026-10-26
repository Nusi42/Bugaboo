//! `bugaboo search-index rebuild`: builds the index of the text search again from the stored
//! texts, for a database that has none yet and after the rule that prepares texts changed (M5
//! WP-05). It works on the stored data and never listens, so it needs no port.

use std::sync::Arc;

use bb_core::message::{self, Message};

use super::StoreArgs;
use super::secrets::open;
use crate::output;

/// Says which database it opened, then reads every issue and makes its rows again. A server that
/// runs meanwhile goes on; its changes reach the index from the outbox as always.
pub(super) async fn rebuild(args: &StoreArgs) -> anyhow::Result<()> {
    let (settings, store) = open(args, "search-index rebuild", None).await?;
    // The settings can name a database by an address (a file of settings, `BUGABOO_DATABASE_URL`):
    // the command says so, never the address itself, which can hold a password.
    let opened: Message = if settings.database_url.is_some() {
        Message::from(message::SEARCH_INDEX_OPENED_ADDRESS)
    } else {
        message::search_index_opened_built_in(settings.data_dir.display())
    };
    output::line(&opened.render());
    let storage = bb_store::AuthStorage::new(store.database, Arc::new(store.keys), store.seal);
    let issues = bb_app::text_index::TextIndexStore::rebuild_text_index(&storage).await?;
    output::line(
        &message::search_index_rebuilt(i64::try_from(issues).unwrap_or(i64::MAX)).render(),
    );
    Ok(())
}
