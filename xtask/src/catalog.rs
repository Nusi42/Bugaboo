//! The check catalogue (`tools/checks/catalog.toml`): the single list of every check.

use std::fs;

use anyhow::Context as _;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use crate::repo::Repo;

/// Where the catalogue lives, relative to the repository root.
pub(crate) const CATALOG_PATH: &str = "tools/checks/catalog.toml";

/// The moments a check can run at, in the order a change travels.
pub(crate) const STAGES: [&str; 6] = ["edit", "commit", "push", "ci", "nightly", "release"];

/// The statuses an entry can have.
pub(crate) const STATUSES: [&str; 2] = ["planned", "active"];

#[derive(Deserialize)]
struct File {
    check: Vec<Entry>,
}

/// One check, as described in the catalogue.
#[derive(Deserialize, Clone)]
pub(crate) struct Entry {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) rule: String,
    pub(crate) tool: String,
    pub(crate) when: Vec<String>,
    pub(crate) files: Vec<String>,
    pub(crate) status: String,
}

impl Entry {
    pub(crate) fn is_active(&self) -> bool {
        self.status == "active"
    }

    pub(crate) fn runs_at(&self, stage: &str) -> bool {
        self.when.iter().any(|when| when == stage)
    }

    /// Whether any of the given repository-relative paths triggers this check.
    pub(crate) fn matches_any(&self, paths: &[String]) -> anyhow::Result<bool> {
        if self.files.iter().any(|glob| glob == "*") {
            return Ok(true);
        }
        let set = self.glob_set()?;
        Ok(paths.iter().any(|path| set.is_match(path)))
    }

    fn glob_set(&self) -> anyhow::Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        for pattern in &self.files {
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .with_context(|| format!("{}: invalid file pattern `{pattern}`", self.id))?;
            builder.add(glob);
        }
        Ok(builder.build()?)
    }
}

/// All entries of the catalogue, in file order.
pub(crate) struct Catalog {
    entries: Vec<Entry>,
}

impl Catalog {
    pub(crate) fn load(repo: &Repo) -> anyhow::Result<Self> {
        let path = repo.path(CATALOG_PATH);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("{CATALOG_PATH} is not valid"))
    }

    pub(crate) fn parse(text: &str) -> anyhow::Result<Self> {
        let file: File = toml::from_str(text)?;
        Ok(Self {
            entries: file.check,
        })
    }

    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub(crate) fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(files: &[&str]) -> Entry {
        Entry {
            id: "T-1".into(),
            title: "t".into(),
            rule: "r".into(),
            tool: "x".into(),
            when: vec!["commit".into()],
            files: files.iter().map(|f| (*f).to_owned()).collect(),
            status: "active".into(),
        }
    }

    #[test]
    fn star_matches_everything() {
        assert!(entry(&["*"]).matches_any(&["a/b.rs".into()]).unwrap());
    }

    #[test]
    fn single_star_does_not_cross_folders() {
        let e = entry(&["crates/*/src/lib.rs"]);
        assert!(
            e.matches_any(&["crates/bb-core/src/lib.rs".into()])
                .unwrap()
        );
        assert!(!e.matches_any(&["crates/a/b/src/lib.rs".into()]).unwrap());
    }

    #[test]
    fn braces_and_double_star_work() {
        let e = entry(&["web/**/*.{ts,tsx}"]);
        assert!(e.matches_any(&["web/app/src/main.tsx".into()]).unwrap());
        assert!(!e.matches_any(&["web/app/src/main.css".into()]).unwrap());
    }

    #[test]
    fn real_catalogue_parses() {
        let text = include_str!("../../tools/checks/catalog.toml");
        let catalog = Catalog::parse(text).unwrap();
        assert!(catalog.get("META-CATALOG").is_some());
    }
}
