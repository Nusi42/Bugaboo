use bb_core::id::UserId;
use bb_core::personal::Entity;

// A record that names a person has a plain title for each of its fields, the message
// `personal-field.<its name>.<the field>`. This one borrows the name of a record that has its own
// name (`personal-kind.draft`), but has a field that no message titles.
#[derive(Entity)]
pub struct Draft {
    #[personal(category = "identity", basis = "contract")]
    #[person(author)]
    pub author: UserId,
    #[not_personal]
    pub shoe_size: u8,
}

fn main() {
    let _ = <Draft as Entity>::MARKS;
}
