use bb_core::event::EntityRef;
use bb_core::id::{EntityId, UserId};
use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Note {
    #[personal(category = "identity", basis = "contract")]
    #[person(author)]
    #[person_for(record, link = "subject")]
    pub author: UserId,
}

fn main() {
    let _: Option<(EntityRef, EntityId)> = None;
}
