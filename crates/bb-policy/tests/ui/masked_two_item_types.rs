use bb_policy::Masked;
use serde::Serialize;

#[derive(Serialize, Masked)]
struct TypeRef {
    #[masked(open)]
    id: String,
}

// A view has one issue type: a second member that says it is the type is refused (ADR-0027).
#[derive(Serialize, Masked)]
struct Item {
    #[masked(nested, item_type)]
    issue_type: TypeRef,
    #[masked(nested, item_type)]
    other_type: TypeRef,
}

fn main() {}
