use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Person {
    #[personal(category = "secrets", basis = "contract")]
    pub email: String,
}

fn main() {}
