use bb_core::plain::Plain;

struct View;

// `Plain` is sealed: a crate cannot declare its own view plain.
impl Plain for View {}

fn main() {}
