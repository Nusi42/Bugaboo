use bb_core::id::IssueId;
use bb_core::personal::Entity;

#[derive(Entity)]
pub struct SearchColumn {
    #[not_personal]
    #[person_for(part_of)]
    pub search: IssueId,
}

fn main() {}
