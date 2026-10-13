//! The roles of a saved layout: the editor offers at most `ROLES_MAX` roles, so a save never
//! changes the others; and the draft holds only what the person sent (M3 WP-09).

use bb_core::access::Role;
use bb_core::id::{LayoutId, RoleId};
use bb_core::layout::LayoutContent;
use jiff::Timestamp;

use super::scenarios::{a_project, token};
use super::scenarios_layouts::{a_layout, lock_the_layout_for, slot_of};
use super::world::{Outcome, World};
use crate::access::{ROLES_MAX, RoleRecord};
use crate::layouts::{FieldInput, SaveLayout, SectionInput, save_layout};

/// Roles beyond the first `ROLES_MAX`, as the last in the order.
fn more_roles(world: &World, count: usize) -> Result<Vec<RoleId>, String> {
    let mut state = world.store.lock();
    let mut made = Vec::new();
    for at in 0..count {
        let last = u8::try_from(at).map_err(|error| error.to_string())?;
        let id = RoleId::from_parts(Timestamp::UNIX_EPOCH, [0, 0, 0, 0, 0, 0, 0, 0, 9, last])
            .map_err(|error| error.to_string())?;
        state.state.roles.push(RoleRecord {
            role: Role {
                id,
                name: format!("Role {at}"),
                starting: None,
                position: 10_000 + i64::try_from(at).map_err(|error| error.to_string())?,
                two_step: false,
                version: 1,
            },
            permissions: std::collections::BTreeSet::new(),
        });
        made.push(id);
    }
    Ok(made)
}

fn stored(world: &World, id: LayoutId) -> Result<LayoutContent, String> {
    world
        .store
        .lock()
        .state
        .layouts
        .iter()
        .find(|found| found.layout.id == id)
        .map(|found| found.content.clone())
        .ok_or_else(|| "no such layout".to_owned())
}

fn inputs(content: &LayoutContent, roles: &dyn Fn(&[RoleId]) -> Vec<RoleId>) -> Vec<SectionInput> {
    content
        .sections
        .iter()
        .map(|section| SectionInput {
            tab: section.tab.clone(),
            title: section.title.clone(),
            fields: section
                .fields
                .iter()
                .map(|placed| FieldInput {
                    field: placed.field,
                    roles: roles(&placed.roles),
                })
                .collect(),
        })
        .collect()
}

#[tokio::test]
async fn a_save_never_takes_the_roles_the_editor_did_not_offer_off_a_field() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "ACME").await?;
    let editor = a_layout(&world, "ACME").await?;
    let slot = slot_of(&editor)?;
    let every = {
        let all = world.store.lock().state.roles.len();
        more_roles(&world, ROLES_MAX + 1 - all)?;
        world.store.lock().state.roles.clone()
    };
    assert_eq!(every.len(), ROLES_MAX + 1);
    let mut ordered: Vec<_> = every
        .iter()
        .map(|record| (record.role.position, record.role.id))
        .collect();
    ordered.sort_unstable();
    let offered: Vec<RoleId> = ordered.iter().take(ROLES_MAX).map(|(_, id)| *id).collect();
    let outside = ordered[ROLES_MAX].1;
    // Every field is shown to every role.
    let everyone: Vec<RoleId> = ordered.iter().map(|(_, id)| *id).collect();
    {
        let mut state = world.store.lock();
        for found in &mut state.state.layouts {
            for section in &mut found.content.sections {
                for placed in &mut section.fields {
                    placed.roles.clone_from(&everyone);
                }
            }
        }
    }
    let content = stored(&world, slot.id)?;
    let lock = lock_the_layout_for(&world, slot, world.admin.person.id);
    // The editor offered the first 100 roles and the person took the first one off every field.
    let without_first = offered[1..].to_vec();
    save_layout(
        &world.ctx(),
        SaveLayout {
            session_token: token(&world),
            layout: slot.id,
            version: slot.version,
            lock_token: lock,
            sections: inputs(&content, &|_| without_first.clone()),
        },
    )
    .await?;
    let after = stored(&world, slot.id)?;
    for placed in after.placed() {
        assert!(placed.roles.contains(&outside), "{:?}", placed.field);
        assert!(!placed.roles.contains(&offered[0]), "{:?}", placed.field);
        assert_eq!(
            placed.roles.len(),
            ROLES_MAX,
            "{:?} missing {:?}",
            placed.field,
            everyone
                .iter()
                .filter(|r| !placed.roles.contains(r))
                .map(|r| ordered.iter().position(|(_, o)| o == r))
                .collect::<Vec<_>>()
        );
    }
    Ok(())
}
