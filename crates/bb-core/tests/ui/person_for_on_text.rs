use bb_core::event::EntityRef;
use bb_core::id::{EntityId, UserId};
use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Comment {
    #[personal(category = "content", basis = "contract")]
    #[person_for(record, link = "subject")]
    pub text: String,
}

fn main() {
    let _: Option<(EntityRef, EntityId, UserId)> = None;
}
