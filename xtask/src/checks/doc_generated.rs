//! DOC-GENERATED: generated files are up to date (SSOT, coding-standards §1).

use super::{Check, Context, Finding};
use crate::generate;
use crate::source::normalize;

pub(crate) struct DocGenerated;

impl Check for DocGenerated {
    fn id(&self) -> &'static str {
        "DOC-GENERATED"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for file in generate::all(context.repo, context.catalog)? {
            let on_disk = std::fs::read_to_string(context.repo.path(file.path)).ok();
            findings.extend(compare(file.path, &file.content, on_disk.as_deref()));
        }
        Ok(findings)
    }
}

/// Compares what the generator produces with what is on disk (line endings do not matter).
fn compare(path: &str, expected: &str, on_disk: Option<&str>) -> Option<Finding> {
    let Some(actual) = on_disk else {
        return Some(Finding::at(
            path,
            "This generated file is missing. Run `cargo xtask docs`, `cargo xtask gen tokens` or `cargo xtask gen messages`, whichever writes it.",
        ));
    };
    (normalize(actual) != expected).then(|| {
        Finding::at(
            path,
            "This generated file is out of date. Edit its source, run the generator (`cargo xtask docs`, `gen tokens` or `gen messages`) and commit the result.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_example() {
        assert!(compare("a.html", "<p>x</p>\n", Some("<p>x</p>\r\n")).is_none());
    }

    #[test]
    fn bad_example() {
        let stale = compare("a.html", "<p>new</p>\n", Some("<p>old</p>\n")).unwrap();
        assert!(stale.message.contains("out of date"));
        let missing = compare("a.html", "<p>new</p>\n", None).unwrap();
        assert!(missing.message.contains("missing"));
    }
}
