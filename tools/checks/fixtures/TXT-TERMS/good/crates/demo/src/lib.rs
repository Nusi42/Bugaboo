//! A crate of the example with names from the Say column.

pub struct AssignedPerson;

pub fn create_project() -> String {
    // A call of a name that other people's code declares is not a name of this code.
    let mut text = String::from("A");
    text.make_ascii_lowercase();
    text
}

/// A word of programming that the glossary does not ask for in code: `ticket` is only banned in
/// what people read.
pub fn ticket_id() {}
