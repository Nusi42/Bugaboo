use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Person {
    #[personal(category = "identity")]
    pub email: String,
}

fn main() {}
