use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Person {
    #[not_personal]
    pub id: u32,
    #[personal(category = "identity", basis = "contract")]
    pub email: String,
}

fn main() {}
