use bb_core::id::UserId;
use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Comment {
    #[personal(category = "identity", basis = "contract")]
    #[person(writer)]
    pub author: UserId,
}

fn main() {}
