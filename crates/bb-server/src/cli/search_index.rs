//! `bugaboo search-index rebuild`: builds the index of the text search again from the stored
//! texts, for a database that has none yet and after the rule that prepares texts changed (M5
//! WP-05). It works on the stored data and never listens, so it needs no port.
//!
//! Like the other commands that look after an installation it migrates nothing: when the data is
//! older or newer than the program it stops and says so (`Database::schema_state`). The index of
//! older data would be made under the old rule and emptied again by the migration at the next
//! start of the server, which builds it again by itself.

use std::sync::Arc;

use bb_core::message::{self, Message};
use bb_store::SchemaState;
use jiff::Timestamp;

use super::StoreArgs;
use super::secrets::open;
use crate::output;

/// Says which database it opened, then builds the index again with the one store function that the
/// start of the server and the sample organization use: the head of the outbox is taken, every
/// issue is read, and the reader is put at that head. A server that runs meanwhile goes on; its
/// changes reach the index from the outbox as always.
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
    match store.database.schema_state().await? {
        SchemaState::Current => {}
        SchemaState::Older => {
            anyhow::bail!(Message::from(message::SEARCH_INDEX_DATA_OLDER).render())
        }
        SchemaState::Newer => {
            anyhow::bail!(Message::from(message::SEARCH_INDEX_DATA_NEWER).render())
        }
    }
    let storage = bb_store::AuthStorage::new(store.database, Arc::new(store.keys), store.seal);
    let issues =
        bb_app::text_index::TextIndexStore::rebuild_text_index(&storage, Timestamp::now()).await?;
    output::line(
        &message::search_index_rebuilt(i64::try_from(issues).unwrap_or(i64::MAX)).render(),
    );
    Ok(())
}
