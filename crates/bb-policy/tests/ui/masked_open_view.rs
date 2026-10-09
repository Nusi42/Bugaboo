use bb_policy::Masked;
use serde::Serialize;

#[derive(Serialize, Masked)]
struct Inner {
    #[masked(field = "title")]
    title: String,
}

// `inside` is a view with a field of the catalogue: calling it `open` would show it to everybody
// who sees `Outer`, so it has to be `nested` (M2-F21).
#[derive(Serialize, Masked)]
struct Outer {
    #[masked(open)]
    inside: Inner,
}

fn main() {}
