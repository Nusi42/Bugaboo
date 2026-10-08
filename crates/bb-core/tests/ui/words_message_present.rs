use bb_core::words;

words! {
    /// A word whose message is in the catalog (`permission-topic.people`).
    pub enum Topic {
        /// People.
        People => "people",
    }
    messages name = "permission-topic.";
}

fn main() {
    assert_eq!(Topic::People.name().as_str(), "permission-topic.people");
}
