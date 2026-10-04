//! Generators: files derived from a single source (coding-standards §1, ADR-0010).
//!
//! Each generator is deterministic and writes already-formatted files, so the format checks pass
//! without a second formatting step.

pub(crate) mod api;
pub(crate) mod contrast_spec;
pub(crate) mod icu;
pub(crate) mod messages;
pub(crate) mod messages_rust;
pub(crate) mod privacy;
pub(crate) mod token_file;
pub(crate) mod tokens;

use anyhow::Context as _;

use crate::catalog::Catalog;
use crate::docs;
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

/// Every generated file in the repository: the docs pages, the token files and the message types.
/// DOC-GENERATED compares all of them with the disk, so a generator is added here.
pub(crate) fn all(repo: &Repo, catalog: &Catalog) -> anyhow::Result<Vec<Generated>> {
    let mut files = docs::generate(repo, catalog)?;
    files.extend(tokens::generate(&token_file::TokenFile::load(repo)?));
    let catalog_text = std::fs::read_to_string(repo.path(messages::SOURCE_LOCALE_PATH))
        .with_context(|| format!("could not read {}", messages::SOURCE_LOCALE_PATH))?;
    files.extend(messages::generate(repo, &catalog_text)?);
    files.extend(privacy::generate(repo)?);
    files.extend(api::generate(repo)?);
    Ok(files)
}

/// The paths of everything [`all`] produces, without producing it. META-COVERAGE counts these as
/// generated.
pub(crate) fn all_paths() -> Vec<&'static str> {
    let mut paths = docs::generated_paths();
    paths.extend(tokens::output_paths());
    paths.extend(messages::output_paths());
    paths.push(privacy::OUTPUT);
    paths.extend(api::output_paths());
    paths
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
