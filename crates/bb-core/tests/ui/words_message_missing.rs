use bb_core::words;

words! {
    /// A word whose message is not in the catalog.
    pub enum Colour {
        /// Red.
        Red => "red",
    }
    messages name = "no-such-prefix.";
}

fn main() {
    let _ = Colour::Red.name();
}
