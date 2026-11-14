use bb_policy::Masked;
use serde::Serialize;

#[derive(Serialize, Masked)]
struct TypeRef {
    #[masked(open)]
    id: String,
}

#[derive(Serialize, Masked)]
struct Typed {
    #[masked(nested, item_type)]
    issue_type: TypeRef,
    #[masked(field = "title")]
    title: String,
}

mod other {
    use bb_policy::Masked;
    use serde::Serialize;

    // Not masked by a type of its own, but it holds a view that is.
    #[derive(Serialize, Masked)]
    pub struct Issue {
        #[masked(nested)]
        pub inner: super::Typed,
    }
}

// A view of another module with the same last name is not the view itself: what it holds counts, as
// for any other member, so the view masked by its type inside it is refused (ADR-0027).
#[derive(Serialize, Masked)]
struct Issue {
    #[masked(nested, item_type)]
    issue_type: TypeRef,
    #[masked(nested)]
    related: Vec<other::Issue>,
}

fn main() {}
