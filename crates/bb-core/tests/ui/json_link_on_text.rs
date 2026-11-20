use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Comment {
    #[personal(category = "content", basis = "contract")]
    #[person_in_json(subject)]
    pub text: String,
}

fn main() {}
