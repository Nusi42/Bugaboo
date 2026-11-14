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

// A view masked by no type of its own that holds items masked by theirs.
#[derive(Serialize, Masked)]
struct Page {
    #[masked(nested)]
    items: Option<Box<Vec<Item>>>,
}

// The items are two levels down, inside `Page`: each would still get the mask of the type of
// `Outer`, so the derive refuses it at any depth (ADR-0027).
#[derive(Serialize, Masked)]
struct Outer {
    #[masked(nested, item_type)]
    issue_type: TypeRef,
    #[masked(nested)]
    page: Page,
}

fn main() {}
