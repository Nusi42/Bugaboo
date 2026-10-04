use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Person {
    #[personal(category = "identity", basis = "contract")]
    pub name: String,
}

#[derive(Entity)]
pub struct Issue {
    #[not_personal]
    pub owner: Person,
}

fn main() {}
