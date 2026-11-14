use bb_policy::Masked;
use serde::Serialize;

#[derive(Serialize, Masked)]
struct TypeRef {
    #[masked(open)]
    id: String,
}

mod other {
    use bb_policy::Masked;
    use serde::Serialize;

    #[derive(Serialize, Masked)]
    pub struct Issue {
        #[masked(nested, item_type)]
        pub issue_type: super::TypeRef,
        #[masked(field = "title")]
        pub title: String,
    }
}

// A view of another module with the same last name is not the view itself: it is masked by its own
// issue type, so the derive refuses it inside a view masked by its type, as any other (ADR-0027).
#[derive(Serialize, Masked)]
struct Issue {
    #[masked(nested, item_type)]
    issue_type: TypeRef,
    #[masked(nested)]
    related: Vec<other::Issue>,
}

fn main() {}
