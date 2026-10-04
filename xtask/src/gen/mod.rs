//! Generators: files derived from a single source (coding-standards §1, ADR-0010).
//!
//! Each generator is deterministic and writes already-formatted files, so the format checks pass
//! without a second formatting step.

pub(crate) mod icu;
pub(crate) mod messages;
pub(crate) mod token_file;
pub(crate) mod tokens;

use anyhow::Context as _;

use crate::repo::Repo;

/// One file a generator produces.
pub(crate) struct Generated {
    /// Repository-relative path with `/`.
    pub(crate) path: &'static str,
    pub(crate) content: String,
}

impl Generated {
    pub(crate) fn new(path: &'static str, content: String) -> Self {
        Self { path, content }
    }
}

/// Writes the files (only when they changed) and returns the paths that were written.
pub(crate) fn write_all(repo: &Repo, files: &[Generated]) -> anyhow::Result<Vec<String>> {
    let mut written = Vec::new();
    for file in files {
        let path = repo.path(file.path);
        if std::fs::read_to_string(&path).is_ok_and(|current| current == file.content) {
            continue;
        }
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)
                .with_context(|| format!("could not create {}", folder.display()))?;
        }
        std::fs::write(&path, &file.content)
            .with_context(|| format!("could not write {}", path.display()))?;
        written.push(file.path.to_owned());
    }
    Ok(written)
}
