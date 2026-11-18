//! `cargo xtask gen keep-rules`: the table of *Keep and delete rules* in the user docs, from
//! `bb_core::keeping::KEEP_KINDS` (ADR-0045, decision 2). DOC-GENERATED fails when the file is not
//! what the code gives.

use super::Generated;

/// Where the table is written, relative to the repository root.
pub(crate) const OUTPUT: &str = "userDocs/privacy/keep-and-delete.md";

/// The page, as it should be.
pub(crate) fn generate() -> Vec<Generated> {
    vec![Generated::new(OUTPUT, bb_core::keeping::keep_table())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priv_05_the_table_has_a_row_for_every_kind() {
        let page = &generate()[0].content;
        for kind in bb_core::keeping::KeepKind::ALL {
            let name = bb_core::message::Message::from(kind.name()).render();
            assert!(page.contains(&name), "{name}");
        }
    }
}
