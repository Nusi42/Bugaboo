use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Person {
    #[not_personal]
    pub id: u32,
    pub email: String,
}

fn main() {}
