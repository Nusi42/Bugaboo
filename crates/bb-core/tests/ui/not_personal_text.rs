use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Issue {
    #[not_personal]
    pub title: String,
}

fn main() {}
