//! LINT-FILE-SIZE: a Rust file has at most 800 lines unless it is named, with a reason, in
//! `tools/checks/size-exemptions.toml` (coding-standards §2).
//!
//! The standard says a file over about 400 lines needs a reason. This check turns the line where a
//! file has surely grown too far into a failure, so a split such as the one WP-02 did cannot come
//! back unseen (`tags/labels.rs` grew from 772 to 1,056 lines in M4). A file over the ceiling is
//! named in the exemption file with the most lines it may have and the follow-up that splits it
//! (or the word *generated*); a file that grows past its cap fails, and an entry for a file that
//! fits again fails, so the list only shrinks.

use serde::Deserialize;

use super::exemptions::{self, Used};
use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::source::read_text;

/// Where the exemptions are written down, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/size-exemptions.toml";

/// The most lines a Rust file has without being named.
const CEILING: usize = 800;

pub(crate) struct LintFileSize;

impl Check for LintFileSize {
    fn id(&self) -> &'static str {
        "LINT-FILE-SIZE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let exemptions: Exemptions = exemptions::load(context.repo, EXEMPTIONS_PATH)?;
        let mut sizes = Vec::new();
        for file in context.repo.all_files()? {
            if has_extension(&file, "rs") {
                let lines = read_text(context.repo, &file)?.lines().count();
                sizes.push((file, lines));
            }
        }
        Ok(evaluate(&sizes, &exemptions))
    }
}

/// The file of exemptions.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exemption>,
    #[serde(default)]
    generated: Vec<Generated>,
}

/// One file that may be longer than the ceiling for now: its path, the most lines it may have, and
/// why, with the follow-up that splits it.
#[derive(Deserialize)]
struct Exemption {
    path: String,
    lines: usize,
    reason: String,
}

/// A file that a tool writes and nobody splits by hand.
#[derive(Deserialize)]
struct Generated {
    path: String,
    reason: String,
}

const UNUSED: &str = "names a file that fits the ceiling, or is gone. Remove it.";

impl exemptions::Exemption for Exemption {
    fn name(&self) -> String {
        self.path.clone()
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn unused(&self) -> String {
        UNUSED.to_owned()
    }
}

impl exemptions::Exemption for Generated {
    fn name(&self) -> String {
        self.path.clone()
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn unused(&self) -> String {
        UNUSED.to_owned()
    }
}

fn evaluate(sizes: &[(String, usize)], exemptions: &Exemptions) -> Vec<Finding> {
    let mut findings = Vec::new();
    let (mut used, mut used_generated) = (Used::default(), Used::default());
    for (path, lines) in sizes.iter().filter(|(_, lines)| *lines > CEILING) {
        if let Some(index) = exemptions
            .generated
            .iter()
            .position(|each| &each.path == path)
        {
            used_generated.mark(index);
            continue;
        }
        let Some(index) = exemptions.exempt.iter().position(|each| &each.path == path) else {
            findings.push(Finding::at(
                path,
                format!(
                    "The file has {lines} lines; the ceiling is {CEILING} (coding-standards §2). Split it by what changes together, or name it in {EXEMPTIONS_PATH} with a reason and the follow-up that splits it."
                ),
            ));
            continue;
        };
        used.mark(index);
        let cap = exemptions.exempt[index].lines;
        if *lines > cap {
            findings.push(Finding::at(
                path,
                format!("The file has {lines} lines, more than the {cap} that {EXEMPTIONS_PATH} allows it. Split it instead of letting it grow."),
            ));
        }
    }
    findings.extend(exemptions::judge(
        EXEMPTIONS_PATH,
        &exemptions.exempt,
        &used,
    ));
    findings.extend(exemptions::follow_up_findings(
        EXEMPTIONS_PATH,
        &exemptions.exempt,
    ));
    findings.extend(exemptions::judge(
        EXEMPTIONS_PATH,
        &exemptions.generated,
        &used_generated,
    ));
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const LONG: &str = "Split in M4-F130 when the file is next touched.";

    fn exemptions(cap: usize) -> Exemptions {
        Exemptions {
            exempt: vec![Exemption {
                path: "a.rs".to_owned(),
                lines: cap,
                reason: LONG.to_owned(),
            }],
            generated: vec![Generated {
                path: "g.rs".to_owned(),
                reason: "Written by cargo xtask gen.".to_owned(),
            }],
        }
    }

    fn sizes(a: usize) -> Vec<(String, usize)> {
        vec![
            ("a.rs".to_owned(), a),
            ("g.rs".to_owned(), 5000),
            ("b.rs".to_owned(), CEILING),
        ]
    }

    #[test]
    fn good_example_files_at_the_ceiling_named_files_and_generated_files() {
        assert!(evaluate(&sizes(900), &exemptions(900)).is_empty());
    }

    #[test]
    fn bad_example_a_file_over_the_ceiling_that_nobody_named() {
        let mut all = sizes(900);
        all.push(("c.rs".to_owned(), CEILING + 1));
        let found = evaluate(&all, &exemptions(900));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location.as_deref(), Some("c.rs"));
    }

    #[test]
    fn bad_example_a_named_file_that_grew_or_that_fits_again() {
        assert_eq!(evaluate(&sizes(901), &exemptions(900)).len(), 1);
        assert_eq!(evaluate(&sizes(CEILING), &exemptions(900)).len(), 1);
    }

    #[test]
    fn bad_example_an_entry_that_names_no_follow_up() {
        let mut named = exemptions(900);
        named.exempt[0].reason = "It is long, and that is fine for now.".to_owned();
        assert_eq!(evaluate(&sizes(900), &named).len(), 1);
    }
}
