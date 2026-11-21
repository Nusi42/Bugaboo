use bb_core::event::EntityRef;
use bb_core::id::{EntityId, UserId};
use bb_core::personal::Entity;

#[derive(Entity)]
pub struct StoredFieldValue {
    #[personal(category = "identity", basis = "contract")]
    #[person_for(field_type = "colleague", key = "field", link = "subject")]
    pub value_ref: Option<EntityId>,
}

fn main() {
    let _: Option<(EntityRef, UserId)> = None;
}
