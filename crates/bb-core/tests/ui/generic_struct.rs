use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Wrapper<T> {
    #[not_personal]
    pub value: T,
}

fn main() {}
