//! What a release carries besides the programs: the parts list (SBOM), the third-party notices
//! and the VEX statements (COMP-01, COMP-02, COMP-03; ADR-0047, decisions 3 and 4).
//!
//! All three are written from the lockfiles, the register and the accepted advisories, never by
//! hand, and in a fixed order, so two runs of one commit write the same bytes.
//!
//! `cargo xtask release` puts them into the release folder with the programs and the notes
//! (`assemble.rs`, `folder.rs`); `release sign` signs it and `release keygen` makes the keys
//! (`sign.rs`); `release check` is the `release` part of DEP-SBOM (SEC-18, ADR-0047 decisions 1
//! and 2).

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::Context as _;
use bb_api_words::about;

mod assemble;
pub(crate) mod folder;
pub(crate) mod formats;
pub(crate) mod notices;
pub(crate) mod parts_list;
pub(crate) mod sign;
pub(crate) mod vex;

/// What `cargo xtask release` is told.
#[derive(clap::Args)]
pub(crate) struct Arguments {
    #[command(subcommand)]
    action: Option<Action>,
    /// Inside WSL: build only the static Linux program, into the folder given with `--into`.
    /// `cargo xtask release` on Windows runs this itself.
    #[arg(long, requires = "into")]
    linux: bool,
    /// The release folder the Linux program goes into (with `--linux`).
    #[arg(long)]
    into: Option<std::path::PathBuf>,
}

#[derive(clap::Subcommand)]
enum Action {
    /// Sign every file of the release folder of this version. The secret key comes from `--key`
    /// or `BUGABOO_RELEASE_KEY`, from a place outside the repository and every work folder; its
    /// passphrase is asked for on the console.
    Sign {
        /// The secret key of the release.
        #[arg(long)]
        key: Option<std::path::PathBuf>,
    },
    /// Check the release folder of this version: every file, its checksum and its signature
    /// against the keys the program carries (the `release` part of DEP-SBOM).
    Check,
    /// Make a key pair for signing releases: the secret key at `--out`, protected by a passphrase
    /// asked for on the console, and the public key beside it (`<out>.pub`).
    Keygen {
        /// Where the secret key goes, outside the repository and every work folder.
        #[arg(long)]
        out: std::path::PathBuf,
    },
}

/// The folder of the release of this version and the name of the release: `<version>`, or
/// `<version>-test` for a test build ([`folder::Build::release_name`]).
fn this_folder(repo: &crate::repo::Repo) -> anyhow::Result<(std::path::PathBuf, String)> {
    let name = assemble::this_build(repo)?.release_name(bb_core::release::VERSION);
    Ok((folder::release_root(repo.root())?.join(&name), name))
}

/// Says what is wrong with a release folder; whether nothing is.
fn report(problems: &[String]) -> bool {
    for problem in problems {
        crate::output::error(problem);
    }
    problems.is_empty()
}

/// `cargo xtask release …`; whether it succeeded.
pub(crate) fn run_release(repo: &crate::repo::Repo, arguments: Arguments) -> anyhow::Result<bool> {
    match (arguments.action, arguments.linux, arguments.into) {
        (None, true, Some(into)) => {
            assemble::assemble_linux(repo, &into)?;
            Ok(true)
        }
        (None, _, _) => {
            let folder = assemble::assemble(repo)?;
            crate::output::info(&format!(
                "wrote the release folder {}; sign it with `cargo xtask release sign`",
                folder.display()
            ));
            Ok(true)
        }
        (Some(Action::Sign { key }), _, _) => {
            let build = assemble::this_build(repo)?;
            if let Some(refusal) = sign::tag_refusal(build, bb_core::release::VERSION) {
                anyhow::bail!(refusal);
            }
            let (folder, name) = this_folder(repo)?;
            let key = sign::open_key(&sign::key_path(key)?, repo.root())?;
            let public = sign::public_of(&key)?;
            if let Some(refusal) = sign::key_refusal(build, &public) {
                anyhow::bail!(refusal);
            }
            if !sign::is_ours(&public) {
                crate::output::note(&format!(
                    "the key {public} is not one of RELEASE_PUBLIC_KEYS, which are test keys: \
                     installed programs will refuse these files"
                ));
            }
            sign::sign_folder(&folder, &name, &key)?;
            // Leaked once for the run of the command, as a key of the list is for the program.
            let keys = [bb_core::release::ReleaseKey {
                name: "signing",
                public: public.leak(),
            }];
            let passed = report(&folder::problems(&folder, &name, &keys));
            if passed {
                crate::output::info(&format!("signed every file of {}", folder.display()));
            }
            Ok(passed)
        }
        (Some(Action::Check), _, _) => {
            let (folder, name) = this_folder(repo)?;
            let passed = report(&folder::problems_with_our_keys(&folder, &name));
            if passed {
                crate::output::info(&format!(
                    "{} holds every file, each signed with a key of the program",
                    folder.display()
                ));
            }
            Ok(passed)
        }
        (Some(Action::Keygen { out }), _, _) => {
            let public = sign::keygen(&out, repo.root(), None)?;
            crate::output::info(&format!(
                "wrote the secret key {} and its public key beside it. Put this public key into \
                 RELEASE_PUBLIC_KEYS (crates/bb-core/src/release/signature.rs):\n{public}",
                out.display()
            ));
            Ok(true)
        }
    }
}

/// The bytes as lower-case hexadecimal.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// A JSON document as the text a release file holds: indented, with a final line break.
pub(crate) fn to_text(document: &serde_json::Value) -> String {
    let mut text = serde_json::to_string_pretty(document).unwrap_or_default();
    text.push('\n');
    text
}

/// Writes `text` to `folder/name`, making the folder first.
pub(crate) fn write_file(folder: &Path, name: &str, text: &str) -> anyhow::Result<()> {
    fs::create_dir_all(folder).with_context(|| format!("could not create {}", folder.display()))?;
    let path = folder.join(name);
    fs::write(&path, text).with_context(|| format!("could not write {}", path.display()))
}

/// Everything made from one state of the repository.
pub(crate) struct Files {
    pub(crate) list: parts_list::PartsList,
    pub(crate) cyclonedx: serde_json::Value,
    pub(crate) spdx: serde_json::Value,
    /// The statements about the accepted advisories (`CycloneDX` VEX).
    pub(crate) vex: serde_json::Value,
    pub(crate) notices: notices::Notices,
}

/// Makes the parts list in both formats and the notices from the repository at `root`.
pub(crate) fn compose(root: &Path) -> anyhow::Result<Files> {
    let list = parts_list::collect(root, &parts_list::commit_time(root))?;
    let register = crate::checks::dep_register::read_register(root)?;
    Ok(Files {
        cyclonedx: formats::cyclonedx(&list),
        spdx: formats::spdx(&list),
        vex: vex::vex(&list, &vex::accepted(root)?, &register),
        notices: notices::notices(root, &list, &register),
        list,
    })
}

/// Writes the parts list in both formats and the notices into `folder`, each file named by
/// `name` from its ending.
fn write_about_files(
    files: &Files,
    folder: &Path,
    name: impl Fn(&str) -> String,
) -> anyhow::Result<()> {
    write_file(
        folder,
        &name(about::CYCLONEDX_END),
        &to_text(&files.cyclonedx),
    )?;
    write_file(folder, &name(about::SPDX_END), &to_text(&files.spdx))?;
    write_file(folder, &name(about::NOTICES_END), &files.notices.text)
}

/// Writes the four files into `folder` under the names of the release named `release`, and
/// nothing else.
pub(crate) fn write(files: &Files, folder: &Path, release: &str) -> anyhow::Result<()> {
    write_about_files(files, folder, |end| about::release_name(release, end))?;
    write_file(folder, &formats::vex_name(release), &to_text(&files.vex))
}

/// Where `cargo xtask parts-list` and the check DEP-SBOM write the files: in the build folder.
pub(crate) fn work_folder(root: &Path) -> anyhow::Result<std::path::PathBuf> {
    Ok(crate::metadata::target_directory(root)?.join("parts-list"))
}

/// The files of this very repository, made once for all the tests that read them.
#[cfg(test)]
pub(crate) fn this_repository() -> &'static Files {
    static FILES: std::sync::OnceLock<Files> = std::sync::OnceLock::new();
    FILES.get_or_init(|| compose(&repository_root()).expect("the parts list of this repository"))
}

#[cfg(test)]
pub(crate) fn repository_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the repository")
        .to_path_buf()
}

/// What is wrong with the two files against their schemas, one line each, naming the file.
pub(crate) fn schema_errors(files: &Files) -> anyhow::Result<Vec<String>> {
    let version = &files.list.program.version;
    let mut lines = Vec::new();
    for error in formats::cyclonedx_errors(&files.cyclonedx)? {
        lines.push(format!("{}: {error}", formats::cyclonedx_name(version)));
    }
    for error in formats::spdx_errors(&files.spdx)? {
        lines.push(format!("{}: {error}", formats::spdx_name(version)));
    }
    for error in formats::cyclonedx_errors(&files.vex)? {
        lines.push(format!("{}: {error}", formats::vex_name(version)));
    }
    Ok(lines)
}

/// Where the program picks up the files it carries (`rust-embed` in `bb-api`, `about.rs`), relative
/// to the repository root. Never committed.
pub(crate) const EMBEDDED_FOLDER: &str = "crates/bb-api/about-files";

/// Writes the parts list in both formats and the notices into `folder`, for the program to take in,
/// and nothing else: files of an earlier version are removed first.
pub(crate) fn write_embedded(files: &Files, folder: &Path) -> anyhow::Result<()> {
    if folder.exists() {
        fs::remove_dir_all(folder)
            .with_context(|| format!("could not clear {}", folder.display()))?;
    }
    write_about_files(files, folder, about::embedded_name)
}

/// `cargo xtask parts-list`: makes the files, checks them against their schemas, and writes them
/// into `out`, else into the work folder.
pub(crate) fn run(root: &Path, out: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    let files = compose(root)?;
    let folder = match out {
        Some(folder) => folder,
        None => work_folder(root)?,
    };
    let invalid = schema_errors(&files)?;
    if !invalid.is_empty() {
        anyhow::bail!(
            "the parts list does not fit its schema:\n{}",
            invalid.join("\n")
        );
    }
    for name in &files.notices.without_text {
        crate::output::info(&format!("no license file in the package of {name}"));
    }
    write(&files, &folder, &files.list.program.version)?;
    crate::output::info(&format!(
        "wrote the parts list ({} parts), the notices and the VEX statements to {}",
        files.list.parts.len(),
        folder.display()
    ));
    Ok(())
}
