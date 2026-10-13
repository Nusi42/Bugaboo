//! The scenarios of the layout use cases (ADR-0007, M1-F18): the preparation, the call, and how
//! many events the call added. `events.rs` names each use case once and runs them.

use bb_core::edit::EditLock;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{EditorId, LockToken};
use bb_core::layout::LayoutKind;
use jiff::Timestamp;

use super::scenarios::{Added, a_project, measured, text, token};
use super::world::World;
use crate::layouts::{
    DeleteLayout, LayoutEditor, LayoutSlot, MakeLayout, SaveLayout, SectionInput, delete_layout,
    make_layout, save_layout,
};

/// The input that makes the layout of the new-issue form of a project.
pub(crate) fn new_layout(world: &World, project: Option<&str>) -> MakeLayout {
    MakeLayout {
        session_token: token(world),
        project: project.map(str::to_owned),
        issue_type: None,
        kind: LayoutKind::NewIssue,
    }
}

/// Makes the layout of a project and says what the editor shows.
pub(crate) async fn a_layout(world: &World, project: &str) -> Result<LayoutEditor, String> {
    Ok(make_layout(&world.ctx(), new_layout(world, Some(project)))
        .await
        .map_err(text)?
        .into_value())
}

/// The slot of an editor that has a layout of its own.
pub(crate) fn slot_of(editor: &LayoutEditor) -> Result<LayoutSlot, String> {
    editor
        .layout
        .ok_or_else(|| "the place has no layout of its own".to_owned())
}

/// The lock Ada holds on a layout, to the end of the century.
pub(crate) fn lock_the_layout(world: &World, slot: LayoutSlot) -> LockToken {
    let token = LockToken::from_random([7; 16]);
    world.store.lock().state.locks.push(EditLock {
        record: EntityRef {
            kind: RecordKind::Layout.into(),
            id: slot.id.into(),
        },
        part: bb_core::technical::TechnicalName::new(bb_core::edit::LAYOUT_PART)
            .unwrap_or_else(|_| unreachable!()),
        holder: world.admin.person.id,
        editor: EditorId::from_random([6; 16]),
        token,
        taken_at: Timestamp::UNIX_EPOCH,
        expires_at: Timestamp::MAX,
    });
    token
}

pub(crate) async fn make_layout_scenario(world: &World) -> Added {
    a_project(world, "ACME").await?;
    measured(
        &world.store,
        make_layout(&world.ctx(), new_layout(world, Some("ACME"))),
    )
    .await
}

pub(crate) async fn save_layout_scenario(world: &World) -> Added {
    a_project(world, "ACME").await?;
    let editor = a_layout(world, "ACME").await?;
    let slot = slot_of(&editor)?;
    let lock = lock_the_layout(world, slot);
    measured(
        &world.store,
        save_layout(
            &world.ctx(),
            SaveLayout {
                session_token: token(world),
                layout: slot.id,
                version: slot.version,
                lock_token: lock,
                sections: vec![SectionInput {
                    tab: String::new(),
                    title: "Only this".to_owned(),
                    fields: Vec::new(),
                }],
            },
        ),
    )
    .await
}

pub(crate) async fn delete_layout_scenario(world: &World) -> Added {
    a_project(world, "ACME").await?;
    let editor = a_layout(world, "ACME").await?;
    let slot = slot_of(&editor)?;
    measured(
        &world.store,
        delete_layout(
            &world.ctx(),
            DeleteLayout {
                session_token: token(world),
                layout: slot.id,
                version: slot.version,
            },
        ),
    )
    .await
}
