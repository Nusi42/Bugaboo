use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Comment {
    #[personal(category = "content", basis = "contract")]
    #[person(author)]
    pub text: String,
}

fn main() {}
