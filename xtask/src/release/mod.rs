//! What a release carries besides the programs: the parts list (SBOM) with the VEX statements in
//! it, and the third-party notices (COMP-01, COMP-02, COMP-03; ADR-0047, decisions 3 and 4, as
//! amended). The program carries the parts list in two formats and the notices for *About*.
//!
//! All of it is written from the lockfiles, the register and the accepted advisories, never by
//! hand, and in a fixed order, so two runs of one commit write the same bytes.
//!
//! `cargo xtask release` puts them into the release folder with the programs and writes the notes
//! beside it (`assemble.rs`, `folder.rs`); `release sign` signs it and `release keygen` makes the
//! keys (`sign.rs`); `release check` is the `release` part of DEP-SBOM (SEC-18, ADR-0047
//! decisions 1 and 2); `release repack` makes the folder from a release of the earlier layout
//! (`repack.rs`).

use std::fs;
use std::path::Path;

use anyhow::Context as _;
use bb_api_words::about;

use crate::checks::sec_build_paths;

pub(crate) mod assemble;
pub(crate) mod folder;
pub(crate) mod formats;
pub(crate) mod notices;
pub(crate) mod parts_list;
pub(crate) mod repack;
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
    /// Sign the release folder of this version: write `SHA256SUMS` and its signature, the only
    /// one. The secret key comes from `--key` or `BUGABOO_RELEASE_KEY`, from a place outside the
    /// repository and every work folder. A key without a passphrase opens without a question; an
    /// older key with one asks for it.
    Sign {
        /// The secret key of the release.
        #[arg(long)]
        key: Option<std::path::PathBuf>,
    },
    /// Check the release folder of this version: its six files and nothing else, every checksum,
    /// the signature of `SHA256SUMS` against the keys the program carries, and the parts list with
    /// its VEX statements against its schema (the `release` part of DEP-SBOM), and that no file
    /// names a folder of a computer (SEC-BUILD-PATHS).
    Check,
    /// Make the release folder of this version from the files of a release published with the
    /// earlier layout (a signature per file, a separate VEX file): the programs and the notices
    /// as they are, the VEX statements put into the `CycloneDX` parts list, nothing else. Sign it
    /// with `release sign` after.
    Repack {
        /// The folder with the published files of the release.
        #[arg(long)]
        from: std::path::PathBuf,
    },
    /// Make a key pair for signing releases: the secret key at `--out`, without a passphrase
    /// and without a question, and the public key beside it (`<out>.pub`).
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

/// Refuses the files at `paths` when one of them names a folder of a computer (SEC-BUILD-PATHS),
/// so that such a file never goes into the release folder or gets signed.
pub(crate) fn refuse_build_paths(root: &Path, paths: &[std::path::PathBuf]) -> anyhow::Result<()> {
    let marks = sec_build_paths::marks(&crate::build_paths::of_this_computer(root)?);
    let mut problems = Vec::new();
    for path in paths {
        if let Some(problem) = sec_build_paths::problem(path, &marks)? {
            let name = path.file_name().unwrap_or(path.as_os_str());
            problems.push(format!("{}: {problem}", name.to_string_lossy()));
        }
    }
    if problems.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "SEC-BUILD-PATHS: these files name folders of a computer, so they are not released or \
         signed:\n{}",
        problems.join("\n")
    )
}

/// The files in the release `folder`, for [`refuse_build_paths`].
fn files_in(folder: &Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in
        fs::read_dir(folder).with_context(|| format!("could not read {}", folder.display()))?
    {
        let path = entry
            .with_context(|| format!("could not read {}", folder.display()))?
            .path();
        if path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
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
            refuse_build_paths(repo.root(), &files_in(&folder)?)?;
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
            refuse_build_paths(repo.root(), &files_in(&folder)?)?;
            sign::sign_folder(&folder, &name, &key)?;
            // Leaked once for the run of the command, as a key of the list is for the program.
            let keys = [bb_core::release::ReleaseKey {
                name: "signing",
                public: public.leak(),
            }];
            let passed = report(&folder::problems(&folder, &name, &keys));
            if passed {
                crate::output::info(&format!(
                    "signed {} with {}",
                    folder.display(),
                    bb_core::release::signature_name(bb_core::release::CHECKSUMS_FILE)
                ));
            }
            Ok(passed)
        }
        (Some(Action::Check), _, _) => {
            let (folder, name) = this_folder(repo)?;
            let clean = match refuse_build_paths(repo.root(), &files_in(&folder)?) {
                Ok(()) => true,
                Err(refusal) => {
                    crate::output::error(&refusal.to_string());
                    false
                }
            };
            let passed = report(&folder::problems_with_our_keys(&folder, &name)) && clean;
            if passed {
                crate::output::info(&format!(
                    "{} holds the files of the release and nothing else, signed with a key of the \
                     program",
                    folder.display()
                ));
            }
            Ok(passed)
        }
        (Some(Action::Repack { from }), _, _) => {
            let (folder, name) = this_folder(repo)?;
            repack::repack(&from, &folder, &name)?;
            crate::output::info(&format!(
                "wrote the release folder {} from {}; sign it with `cargo xtask release sign`",
                folder.display(),
                from.display()
            ));
            Ok(true)
        }
        (Some(Action::Keygen { out }), _, _) => {
            let public = sign::keygen(&out, repo.root())?;
            crate::output::info(&format!(
                "wrote the secret key {} and its public key beside it. Put this public key into \
                 RELEASE_PUBLIC_KEYS (crates/bb-core/src/release/signature.rs):\n{public}",
                out.display()
            ));
            Ok(true)
        }
    }
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
    /// The parts list as `CycloneDX`, with the statements about the accepted advisories (VEX).
    pub(crate) cyclonedx: serde_json::Value,
    pub(crate) spdx: serde_json::Value,
    pub(crate) notices: notices::Notices,
}

/// Makes the parts list in both formats, the `CycloneDX` one with the VEX statements, and the
/// notices from the repository at `root`.
pub(crate) fn compose(root: &Path) -> anyhow::Result<Files> {
    let list = parts_list::collect(root, &parts_list::commit_time(root))?;
    let register = crate::checks::dep_register::read_register(root)?;
    let statements = vex::statements(&list, &vex::accepted(root)?, &register);
    Ok(Files {
        cyclonedx: vex::with_statements(formats::cyclonedx(&list), statements),
        spdx: formats::spdx(&list),
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

/// Writes the parts list in both formats and the notices into `folder` under the names of the
/// release named `release`, and nothing else (`cargo xtask parts-list`, DEP-SBOM).
pub(crate) fn write(files: &Files, folder: &Path, release: &str) -> anyhow::Result<()> {
    write_about_files(files, folder, |end| about::release_name(release, end))
}

/// Writes the files of the release named `release` that are made from the repository into its
/// folder: the `CycloneDX` parts list and the notices ([`folder::payload`]).
pub(crate) fn write_release(files: &Files, folder: &Path, release: &str) -> anyhow::Result<()> {
    write_file(
        folder,
        &formats::cyclonedx_name(release),
        &to_text(&files.cyclonedx),
    )?;
    write_file(
        folder,
        &about::release_name(release, about::NOTICES_END),
        &files.notices.text,
    )
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

/// What is wrong with the two formats of the parts list against their schemas, one line each,
/// naming the file.
pub(crate) fn schema_errors(files: &Files) -> anyhow::Result<Vec<String>> {
    let version = &files.list.program.version;
    let mut lines = Vec::new();
    for error in formats::cyclonedx_errors(&files.cyclonedx)? {
        lines.push(format!("{}: {error}", formats::cyclonedx_name(version)));
    }
    for error in formats::spdx_errors(&files.spdx)? {
        lines.push(format!("{}: {error}", formats::spdx_name(version)));
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
pub(crate) fn run(root: &Path, out: Option<std::path::PathBuf>, embed: bool) -> anyhow::Result<()> {
    let files = compose(root)?;
    if embed {
        write_embedded(&files, &root.join(EMBEDDED_FOLDER))?;
        crate::output::info(&format!(
            "put the parts list ({} parts) and the notices where the program takes them in",
            files.list.parts.len()
        ));
        return Ok(());
    }
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
        "wrote the parts list ({} parts) with the VEX statements, and the notices, to {}",
        files.list.parts.len(),
        folder.display()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_18_a_file_that_names_a_folder_of_a_computer_is_never_released() {
        let folder = tempfile::tempdir().unwrap();
        let clean = folder.path().join("bugaboo-0.6.0-linux-x64");
        fs::write(&clean, b"\0/cargo/registry/src/index/a-1.0.0/src/lib.rs\0").unwrap();
        let root = repository_root();
        refuse_build_paths(&root, &files_in(folder.path()).unwrap()).unwrap();
        let planted = folder.path().join("bugaboo-0.6.0-windows-x64.exe");
        fs::write(
            &planted,
            br"MZ\0C:\Users\someone\.cargo\registry\src\index\a-1.0.0\src\lib.rs\0",
        )
        .unwrap();
        let refusal = refuse_build_paths(&root, &files_in(folder.path()).unwrap())
            .unwrap_err()
            .to_string();
        assert!(refusal.starts_with("SEC-BUILD-PATHS"), "{refusal}");
        assert!(
            refusal.contains("bugaboo-0.6.0-windows-x64.exe: names folders"),
            "{refusal}"
        );
        assert!(!refusal.contains("linux-x64"), "{refusal}");
        assert!(!refusal.contains("someone"), "{refusal}");
    }
}
