use bb_core::event::EntityRef;
use bb_core::id::{EntityId, UserId};
use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Note {
    #[not_personal]
    #[person_for(record)]
    pub about: EntityRef,
}

fn main() {
    let _: Option<(EntityId, UserId)> = None;
}
