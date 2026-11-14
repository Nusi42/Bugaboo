use bb_policy::Masked;
use serde::Serialize;

#[derive(Serialize, Masked)]
struct TypeRef {
    #[masked(open)]
    id: String,
}

#[derive(Serialize, Masked)]
struct Item {
    #[masked(nested, item_type)]
    issue_type: TypeRef,
    #[masked(field = "title")]
    title: String,
}

// An item masked by its own issue type holds items that are masked by theirs: each related item
// would get the mask of the type of `Outer`, not of its own, so the derive refuses it (ADR-0027).
#[derive(Serialize, Masked)]
struct Outer {
    #[masked(nested, item_type)]
    issue_type: TypeRef,
    #[masked(nested)]
    related: Vec<Item>,
}

fn main() {}
