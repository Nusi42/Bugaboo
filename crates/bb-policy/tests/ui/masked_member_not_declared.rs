use bb_policy::Masked;
use serde::Serialize;

#[derive(Serialize, Masked)]
struct Inner {
    #[masked(field = "title")]
    title: String,
}

// `inside` holds a view with a field of the catalogue but says nothing about how it is masked: a
// view that holds another view cannot be written without saying so (M2-F21).
#[derive(Serialize, Masked)]
struct Outer {
    #[masked(open)]
    id: u32,
    inside: Inner,
}

fn main() {}
