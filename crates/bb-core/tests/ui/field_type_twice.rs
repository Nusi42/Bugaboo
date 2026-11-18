use bb_core::event::EntityRef;
use bb_core::id::{EntityId, UserId};
use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Note {
    #[personal(category = "identity", basis = "contract")]
    #[person_for(field_type = "person", key = "field", link = "subject")]
    #[person_for(field_type = "person", key = "field", link = "author")]
    pub value: Option<EntityId>,
}

fn main() {
    let _: Option<(EntityRef, UserId)> = None;
}
