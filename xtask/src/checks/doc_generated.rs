//! DOC-GENERATED: generated files are up to date (SSOT, coding-standards §1).

use super::{Check, Context, Finding};
use crate::generate;
use crate::source::{normalize, read_text};

pub(crate) struct DocGenerated;

impl Check for DocGenerated {
    fn id(&self) -> &'static str {
        "DOC-GENERATED"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for generator in &generate::GENERATORS {
            for file in (generator.generate)(context.repo, context.catalog)? {
                let finding = match &file.bytes {
                    Some(bytes) => compare_bytes(
                        generator.name,
                        file.path,
                        bytes,
                        std::fs::read(context.repo.path(file.path)).ok().as_deref(),
                    ),
                    None => compare(
                        generator.name,
                        file.path,
                        &file.content,
                        read_text(context.repo, file.path).ok().as_deref(),
                    ),
                };
                findings.extend(finding);
            }
        }
        Ok(findings)
    }
}

/// Compares what the generator called `generator` produces with what is on disk (line endings do
/// not matter).
fn compare(generator: &str, path: &str, expected: &str, on_disk: Option<&str>) -> Option<Finding> {
    let Some(actual) = on_disk else {
        return Some(missing(generator, path));
    };
    (normalize(actual) != expected).then(|| stale(generator, path))
}

/// Compares a generated picture with the one on disk, byte for byte.
fn compare_bytes(
    generator: &str,
    path: &str,
    expected: &[u8],
    on_disk: Option<&[u8]>,
) -> Option<Finding> {
    let Some(actual) = on_disk else {
        return Some(missing(generator, path));
    };
    (actual != expected).then(|| stale(generator, path))
}

fn missing(generator: &str, path: &str) -> Finding {
    Finding::at(
        path,
        format!("This generated file is missing. Run `cargo xtask gen {generator}`."),
    )
}

fn stale(generator: &str, path: &str) -> Finding {
    Finding::at(
        path,
        format!(
            "This generated file is out of date. Edit its source, run `cargo xtask gen {generator}` and commit the result."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_example() {
        assert!(compare("docs", "a.html", "<p>x</p>\n", Some("<p>x</p>\r\n")).is_none());
    }

    #[test]
    fn bad_example() {
        let stale = compare("docs", "a.html", "<p>new</p>\n", Some("<p>old</p>\n")).unwrap();
        assert!(stale.message.contains("out of date"));
        let missing = compare("docs", "a.html", "<p>new</p>\n", None).unwrap();
        assert!(missing.message.contains("missing"));
        assert!(missing.message.contains("`cargo xtask gen docs`"));
        let other = compare("roles", "a.md", "x\n", None).unwrap();
        assert!(other.message.contains("`cargo xtask gen roles`"));
    }

    #[test]
    fn a_picture_is_compared_byte_for_byte() {
        assert!(compare_bytes("theme", "a.png", b"\x89PNG\r\n", Some(b"\x89PNG\r\n")).is_none());
        let changed = compare_bytes("theme", "a.png", b"\x89PNG\r\n", Some(b"\x89PNG\n")).unwrap();
        assert!(changed.message.contains("out of date"));
        let missing = compare_bytes("theme", "a.png", b"\x89PNG", None).unwrap();
        assert!(missing.message.contains("missing"));
    }
}
