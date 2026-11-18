use bb_core::id::UserId;
use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Note {
    #[personal(category = "identity", basis = "contract")]
    #[person(author)]
    #[person(holder)]
    pub author: UserId,
}

fn main() {}
