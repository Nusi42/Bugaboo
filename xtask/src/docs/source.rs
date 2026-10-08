//! Where the documentation sources live, and reading them. Every other module asks here.

use std::fs;

use anyhow::Context as _;

use super::text::has_extension;
use crate::repo::Repo;
use crate::source::read_text;

pub(crate) const ARCHITECTURE_DIR: &str = "docs/architecture";
pub(crate) const ADR_DIR: &str = "docs/adr";
pub(crate) const MILESTONE_PACKAGES_DIR: &str = "docs/milestones";
pub(crate) const FEATURE_LIST: &str = "docs/01-feature-list.md";
pub(crate) const MILESTONES: &str = "docs/04-milestones.md";
pub(crate) const CODING_STANDARDS: &str = "docs/coding-standards.md";
pub(crate) const BUILDING_BLOCKS: &str = "docs/architecture/04-building-blocks.md";
pub(crate) const ARCHITECTURE_README: &str = "docs/architecture/README.md";

/// The names of the files in a folder (relative to the repository root) whose name passes
/// `wanted`, sorted.
pub(crate) fn sorted_files(
    repo: &Repo,
    folder: &str,
    wanted: impl Fn(&str) -> bool,
) -> anyhow::Result<Vec<String>> {
    let path = repo.path(folder);
    let mut names = Vec::new();
    for entry in
        fs::read_dir(&path).with_context(|| format!("could not read folder {}", path.display()))?
    {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if wanted(&name) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn starts_with_digits(name: &str, count: usize) -> bool {
    name.len() > count + 1
        && name.as_bytes()[..count].iter().all(u8::is_ascii_digit)
        && name.as_bytes()[count] == b'-'
        && has_extension(name, "md")
}

/// The architecture chapter files (`04-building-blocks.md`), in order.
pub(crate) fn chapter_files(repo: &Repo) -> anyhow::Result<Vec<String>> {
    sorted_files(repo, ARCHITECTURE_DIR, |name| starts_with_digits(name, 2))
}

/// The decision record files (`0020-architecture-checks.md`), in order.
pub(crate) fn adr_files(repo: &Repo) -> anyhow::Result<Vec<String>> {
    sorted_files(repo, ADR_DIR, |name| starts_with_digits(name, 4))
}

/// Reads every decision record: `(path, text)`.
pub(crate) fn read_adrs(repo: &Repo) -> anyhow::Result<Vec<(String, String)>> {
    adr_files(repo)?
        .into_iter()
        .map(|name| {
            let path = format!("{ADR_DIR}/{name}");
            let text = read_text(repo, &path)?;
            Ok((path, text))
        })
        .collect()
}

/// Reads every architecture chapter: `(path, text)`.
pub(crate) fn read_chapters(repo: &Repo) -> anyhow::Result<Vec<(String, String)>> {
    chapter_files(repo)?
        .into_iter()
        .map(|name| {
            let path = format!("{ARCHITECTURE_DIR}/{name}");
            let text = read_text(repo, &path)?;
            Ok((path, text))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_names_are_recognised() {
        assert!(starts_with_digits("04-building-blocks.md", 2));
        assert!(!starts_with_digits("README.md", 2));
        assert!(starts_with_digits("0020-architecture-checks.md", 4));
        assert!(!starts_with_digits("04-x.md", 4));
    }
}
