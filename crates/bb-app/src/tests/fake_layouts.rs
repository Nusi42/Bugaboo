//! The layouts part of the in-memory store.

use bb_core::id::LayoutId;
use bb_core::layout::{Layout, LayoutContent, LayoutKind, StoredLayout};

use super::fake::{FakeError, FakeStore, Tx};
use crate::layouts::LayoutStore;

impl LayoutStore for FakeStore {
    async fn layout_headers_in(tx: &mut Tx, kind: LayoutKind) -> Result<Vec<Layout>, FakeError> {
        let mut found: Vec<Layout> = tx
            .state
            .layouts
            .iter()
            .filter(|stored| stored.layout.kind == kind)
            .map(|stored| stored.layout.clone())
            .collect();
        found.sort_by_key(|layout| layout.id);
        Ok(found)
    }

    async fn stored_layout_in(
        tx: &mut Tx,
        id: LayoutId,
    ) -> Result<Option<StoredLayout>, FakeError> {
        Ok(tx
            .state
            .layouts
            .iter()
            .find(|stored| stored.layout.id == id)
            .cloned())
    }

    async fn insert_layout(
        tx: &mut Tx,
        layout: &Layout,
        content: &LayoutContent,
    ) -> Result<(), FakeError> {
        tx.state.layouts.push(StoredLayout {
            layout: layout.clone(),
            content: content.clone(),
        });
        Ok(())
    }

    async fn replace_layout(
        tx: &mut Tx,
        id: LayoutId,
        content: &LayoutContent,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .layouts
            .iter_mut()
            .find(|stored| stored.layout.id == id && stored.layout.version == version)
        else {
            return Ok(false);
        };
        found.content = content.clone();
        found.layout.version += 1;
        Ok(true)
    }

    async fn delete_layout(tx: &mut Tx, id: LayoutId, version: i64) -> Result<bool, FakeError> {
        let before = tx.state.layouts.len();
        tx.state
            .layouts
            .retain(|stored| !(stored.layout.id == id && stored.layout.version == version));
        Ok(tx.state.layouts.len() != before)
    }
}
