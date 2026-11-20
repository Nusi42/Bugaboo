use bb_core::id::UserId;
use bb_core::personal::Entity;

// A kind of record that names a person has no message `personal-kind.note`, so it has no plain
// name for *Download my data* and *Remove a person*.
#[derive(Entity)]
pub struct Note {
    #[personal(category = "identity", basis = "contract")]
    #[person(author)]
    pub author: UserId,
}

fn main() {
    let _ = <Note as Entity>::MARKS;
}
