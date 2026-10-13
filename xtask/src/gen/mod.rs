//! Generators: files derived from a single source (coding-standards §1, ADR-0010).
//!
//! Each generator is deterministic and writes already-formatted files, so the format checks pass
//! without a second formatting step.

pub(crate) mod api;
pub(crate) mod clippy;
pub(crate) mod contrast_spec;
pub(crate) mod icu;
pub(crate) mod kdl_reader;
pub(crate) mod made_keys;
pub(crate) mod messages;
pub(crate) mod messages_rust;
pub(crate) mod privacy;
pub(crate) mod roles;
pub(crate) mod theme;
pub(crate) mod theme_file;
pub(crate) mod theme_selector;

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

/// One generator: the files it writes, and how it makes them.
pub(crate) struct Generator {
    /// The name that `cargo xtask gen <name>` takes.
    pub(crate) name: &'static str,
    /// What it does, in one line.
    pub(crate) help: &'static str,
    /// The paths it writes, without producing them. META-COVERAGE counts these as generated.
    pub(crate) paths: fn() -> Vec<&'static str>,
    /// The files, as they should be.
    pub(crate) generate: fn(&Repo, &Catalog) -> anyhow::Result<Vec<Generated>>,
}

/// The name of the generator that builds the HTML pages (`cargo xtask docs` is its short form).
pub(crate) const DOCS: &str = "docs";

/// Every generator, once. A generated file in the repository is written by one of these;
/// DOC-GENERATED compares all of them with the disk, `cargo xtask gen <name>` writes one, and
/// META-COVERAGE counts their paths as generated.
pub(crate) const GENERATORS: [Generator; 7] = [
    Generator {
        name: DOCS,
        help: "The commentable HTML pages from the Markdown sources and the check catalogue.",
        paths: docs::generated_paths,
        generate: docs::generate,
    },
    Generator {
        name: "theme",
        help: "The style sheet, the TypeScript names and the Rust theme choices from design/theme.kdl.",
        paths: theme::output_paths,
        generate: |repo, _| theme::outputs(repo),
    },
    Generator {
        name: "messages",
        help: "Typed message keys from messages/en.json.",
        paths: messages::output_paths,
        generate: |repo, _| messages::outputs(repo),
    },
    Generator {
        name: "privacy",
        help: "The personal-data page docs/privacy/personal-data.md from the marks in the code.",
        paths: || vec![privacy::OUTPUT],
        generate: |repo, _| privacy::generate(repo),
    },
    Generator {
        name: "roles",
        help: "The tables of permissions and starting roles in architecture section 7, from bb-policy.",
        paths: || vec![roles::PAGE],
        generate: |repo, _| roles::generate(repo),
    },
    Generator {
        name: "clippy",
        help: "The clippy.toml of the crates that do no I/O, from the root file and the list of what reaches the world.",
        paths: clippy::output_paths,
        generate: |repo, _| clippy::generate(repo),
    },
    Generator {
        name: "api",
        help: "The API description, the browser client's types and the cookie list, from bb-api.",
        paths: api::output_paths,
        generate: |repo, _| api::generate(repo),
    },
];

/// The names of the generators with their help, for the command line.
pub(crate) fn possible_values() -> clap::builder::PossibleValuesParser {
    clap::builder::PossibleValuesParser::new(
        GENERATORS.iter().map(|generator| {
            clap::builder::PossibleValue::new(generator.name).help(generator.help)
        }),
    )
}

/// The generator called `name`.
pub(crate) fn named(name: &str) -> anyhow::Result<&'static Generator> {
    GENERATORS
        .iter()
        .find(|generator| generator.name == name)
        .with_context(|| format!("there is no generator called `{name}`"))
}

/// The paths of everything the generators produce, without producing it.
pub(crate) fn all_paths() -> Vec<&'static str> {
    GENERATORS
        .iter()
        .flat_map(|generator| (generator.paths)())
        .collect()
}

/// Writes the files of the generator called `name`; returns the paths that changed.
pub(crate) fn write_named(
    repo: &Repo,
    catalog: &Catalog,
    name: &str,
) -> anyhow::Result<Vec<String>> {
    let generator = named(name)?;
    write_all(repo, &(generator.generate)(repo, catalog)?)
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
