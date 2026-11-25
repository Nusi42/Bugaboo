//! `cargo xtask release`: builds the release folder of this version (ADR-0047, decision 1).
//!
//! On Windows it builds the program with `cargo xtask build` (the parts list and the notices are
//! made fresh and taken into the program first), runs the same command with `--linux` inside WSL
//! for the static Linux program, writes the parts list with the VEX statements and the notices,
//! writes the release notes beside the folder, and leaves the folder for `cargo xtask release
//! sign`. A folder for a version
//! whose tag `v<version>` is not on the commit, or of a program that trusts only test keys, is a
//! test build: its name, the names of its files and its notes say so. When the tag exists, the
//! day of the release in `Cargo.toml` must be the day of the tag's commit.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, bail};
use bb_api_words::about;
use bb_core::release;
use jiff::civil::Date;

use super::folder::{self, Build, LINUX, PLATFORMS, Platform};
use crate::generate::release as parts;
use crate::repo::Repo;
use crate::steps::{self, Step};
use crate::wsl;

/// What the first comment of a part of the release notes says besides its name.
const NOTE: &str = "by `cargo xtask release` from bb-core and Cargo.toml -->";

/// The name of the part of the notes that says whether this is a test build.
const BUILD_PART: &str = "build";

/// Where the notes of a version are written by hand, and the template a version without its own
/// notes starts from.
const NOTES_FOLDER: &str = "docs/releases";
const NOTES_TEMPLATE: &str = "TEMPLATE.md";

/// The tag of the release `version`.
fn tag_of(version: &str) -> String {
    format!("v{version}")
}

/// Whether the tag `v<version>` is on the commit that is checked out.
pub(crate) fn tagged(repo: &Repo, version: &str) -> anyhow::Result<bool> {
    let tag = tag_of(version);
    Ok(repo
        .git_lines(&["tag", "--points-at", "HEAD"])?
        .iter()
        .any(|line| line.trim() == tag))
}

/// The build of this program from the commit that is checked out.
pub(crate) fn this_build(repo: &Repo) -> anyhow::Result<Build> {
    Ok(Build::of_this_program(tagged(repo, release::VERSION)?))
}

/// The day of the commit the tag `v<version>` names, `None` when there is no such tag.
fn tag_day(repo: &Repo, version: &str) -> anyhow::Result<Option<Date>> {
    let tag = tag_of(version);
    if repo.git_lines(&["tag", "--list", &tag])?.is_empty() {
        return Ok(None);
    }
    let commit = format!("{tag}^{{commit}}");
    let day = repo.git_lines(&["log", "-1", "--format=%cs", &commit])?;
    let day = day
        .first()
        .with_context(|| format!("Git named no day for the commit of {tag}"))?;
    Ok(Some(day.parse().with_context(|| {
        format!("Git named the day of {tag} as {day}")
    })?))
}

/// Why the day of the release in `Cargo.toml` (`released`) cannot be the release of `version`, if
/// it cannot: its tag exists and names a commit of another day (`tagged_on`). Without the tag
/// (a test build) any day is taken.
fn date_problem(version: &str, released: Date, tagged_on: Option<Date>) -> Option<String> {
    let tagged_on = tagged_on?;
    (tagged_on != released).then(|| {
        format!(
            "the release date in Cargo.toml is {released}, but the commit of the tag {} is of \
             {tagged_on}: write the day of the tag's commit into [workspace.metadata.release].",
            tag_of(version)
        )
    })
}

/// Why the files the program takes in cannot be the ones of `version`, if they cannot: the folder
/// is missing, or its parts list names another version. `cargo xtask build` writes them fresh, so
/// this holds right after it; a program built another way lists no parts (WP-05).
pub(crate) fn embedded_problem(root: &Path, version: &str) -> Option<String> {
    let path = root
        .join(super::EMBEDDED_FOLDER)
        .join(about::embedded_name(about::CYCLONEDX_END));
    let Ok(text) = fs::read_to_string(&path) else {
        return Some(format!(
            "{} is missing: the program would carry no parts list. Run `cargo xtask build`.",
            path.display()
        ));
    };
    let named = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|document| {
            document
                .pointer("/metadata/component/version")
                .and_then(|value| value.as_str().map(str::to_owned))
        });
    (named.as_deref() != Some(version)).then(|| {
        format!(
            "{} names version {}, not {version}: the program would carry another version's parts list.",
            path.display(),
            named.unwrap_or_default()
        )
    })
}

/// The text of the notes: the version's own (`docs/releases/<version>.md`) or the template, with
/// the keys, the support period and the kind of build written in.
fn notes(root: &Path, version: &str, build: Build) -> anyhow::Result<String> {
    let own = format!("{NOTES_FOLDER}/{version}.md");
    let template = format!("{NOTES_FOLDER}/{NOTES_TEMPLATE}");
    let path = if root.join(&own).is_file() {
        own
    } else {
        template
    };
    let mut text = fs::read_to_string(root.join(&path))
        .with_context(|| format!("could not read {path}"))?
        .replace("{version}", version);
    for (name, part) in parts::parts(version) {
        text = crate::generate::replace_part(&path, &text, name, NOTE, &part)?;
    }
    let kind = match (build.tagged, build.test_keys) {
        (true, false) => "This is the release of the tag `v{version}`.".to_owned(),
        (false, _) => format!(
            "**This is a test build.** The tag `v{version}` is not on the commit it was built from; \
             do not install it on a server people use."
        ),
        (true, true) => format!(
            "**This release is signed with a test key.** The tag `v{version}` is on its commit, but \
             the program trusts test keys only; do not install it on a server people use."
        ),
    }
    .replace("{version}", version);
    crate::generate::replace_part(&path, &text, BUILD_PART, NOTE, &kind)
}

/// The program `cargo xtask build` made for `platform`, under the build folder.
fn built_program(root: &Path, platform: Platform) -> anyhow::Result<PathBuf> {
    let mut path = crate::metadata::target_directory(root)?;
    if let Some(target) = platform.target {
        path.push(target);
    }
    path.push("release");
    path.push(format!("{}{}", steps::SERVER_BINARY, platform.end));
    Ok(path)
}

/// Builds the program for `platform` with `cargo xtask build` and copies it into `folder`, under
/// the name of the release `release`.
fn build_into(root: &Path, platform: Platform, folder: &Path, release: &str) -> anyhow::Result<()> {
    if !crate::build::run(root, platform.target)? {
        bail!("the build of the {} program failed", platform.word);
    }
    if let Some(problem) = embedded_problem(root, release::VERSION) {
        bail!(problem);
    }
    let program = built_program(root, platform)?;
    super::refuse_build_paths(root, std::slice::from_ref(&program))?;
    let name = folder::program_name(release, platform);
    fs::copy(&program, folder.join(&name))
        .with_context(|| format!("could not copy {} to {name}", program.display()))?;
    Ok(())
}

/// The step that runs `cargo xtask release --linux --into <folder>` in WSL, from the same folder,
/// with a build folder of its own (`target/wsl`), so the two systems never share compiled files.
/// It runs in `distribution` when the checkout names one ([`wsl::build_distribution`]), else in
/// the default distribution.
pub(crate) fn wsl_step(root: &Path, distribution: Option<&str>, folder_in_wsl: &str) -> Step {
    let command = format!(
        "CARGO_TARGET_DIR=target/wsl cargo xtask release --linux --into '{}'",
        folder_in_wsl.replace('\'', r"'\''")
    );
    let mut step = Step::new("build the Linux program in WSL", wsl::PROGRAM, &[]);
    step.args = wsl::arguments(
        distribution,
        &[
            "--cd",
            &root.to_string_lossy(),
            "--exec",
            "bash",
            "-lc",
            &command,
        ],
    );
    if !cfg!(windows) {
        step.skip = Some("only on Windows; on Linux run `cargo xtask release --linux`");
    }
    step
}

/// The path of a Windows folder as the WSL `distribution` sees it (the default one for `None`).
pub(crate) fn wsl_path(folder: &Path, distribution: Option<&str>) -> anyhow::Result<String> {
    let output = Command::new(wsl::PROGRAM)
        .args(wsl::arguments(
            distribution,
            &["--exec", "wslpath", "-a", "-u"],
        ))
        .arg(folder)
        .output()
        .context("could not start wsl.exe: is WSL installed?")?;
    if !output.status.success() {
        bail!("WSL could not name the folder {}", folder.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// `cargo xtask release`: the folder of this version, built on Windows and in WSL, unsigned.
pub(crate) fn assemble(repo: &Repo) -> anyhow::Result<PathBuf> {
    let root = repo.root();
    let version = release::VERSION;
    if let Some(problem) = date_problem(version, release::RELEASE_DATE, tag_day(repo, version)?) {
        bail!(problem);
    }
    let build = this_build(repo)?;
    let name = build.release_name(version);
    let folder = folder::release_root(root)?.join(&name);
    if folder.exists() {
        fs::remove_dir_all(&folder)
            .with_context(|| format!("could not clear {}", folder.display()))?;
    }
    fs::create_dir_all(&folder)
        .with_context(|| format!("could not create {}", folder.display()))?;
    // The Linux program first: its build in WSL writes the files the program takes in too, and the
    // Windows build after it leaves them as the Windows program has them.
    let distribution = wsl::build_distribution(root);
    let distribution = distribution.as_deref();
    let in_wsl = wsl_path(&folder, distribution)?;
    if !steps::run(root, &[wsl_step(root, distribution, &in_wsl)])?.passed {
        bail!("the Linux program could not be built in WSL");
    }
    build_into(root, PLATFORMS[0], &folder, &name)?;
    let files = super::compose(root)?;
    super::write_release(&files, &folder, &name)?;
    let notes_path = folder::notes_path(&folder, &name);
    fs::write(&notes_path, notes(root, version, build)?)
        .with_context(|| format!("could not write {}", notes_path.display()))?;
    Ok(folder)
}

/// `cargo xtask release --linux --into <folder>`, inside WSL: the static Linux program into the
/// folder that the Windows side made, under the name of the release that folder is for.
pub(crate) fn assemble_linux(repo: &Repo, into: &Path) -> anyhow::Result<()> {
    if !cfg!(target_os = "linux") {
        bail!("`--linux` builds the Linux program and runs only on Linux (in WSL)");
    }
    let name = release_name_of(into, release::VERSION)?;
    build_into(repo.root(), LINUX, into, &name)
}

/// The name of the release a folder is for: the folder's own name, which the Windows side gave it
/// with [`Build::release_name`]. Git is asked on Windows only, so WSL never reads the tags of a
/// checkout on another system; a folder that is not named for a release of `version` is refused.
fn release_name_of(folder: &Path, version: &str) -> anyhow::Result<String> {
    let name = folder
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let names = [true, false].map(|tagged| {
        Build {
            tagged,
            test_keys: false,
        }
        .release_name(version)
    });
    if !names.iter().any(|each| each == name) {
        bail!(
            "{} is not the folder of a release of {version}: its name must be {}",
            folder.display(),
            names.join(" or ")
        );
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_18_the_linux_program_is_built_in_wsl_with_a_build_folder_of_its_own() {
        let step = wsl_step(Path::new("C:/work/bugaboo"), None, "/mnt/c/work/it's");
        assert_eq!(step.program, "wsl.exe");
        // Without a named distribution the default one builds: no `-d`.
        assert_eq!(step.args[0], "--cd", "{:?}", step.args);
        assert!(!step.args.iter().any(|argument| argument == "-d"));
        let command = step.args.last().unwrap();
        assert!(command.starts_with("CARGO_TARGET_DIR=target/wsl cargo xtask release --linux"));
        assert!(
            command.ends_with(r"--into '/mnt/c/work/it'\''s'"),
            "{command}"
        );
    }

    #[test]
    fn sec_18_the_linux_program_is_built_in_the_distribution_the_checkout_names() {
        let root = Path::new("C:/work/bugaboo");
        let default = wsl_step(root, None, "/mnt/c/work/out");
        let named = wsl_step(root, Some("builder"), "/mnt/c/work/out");
        assert_eq!(named.program, "wsl.exe");
        assert_eq!(named.args[..2], ["-d", "builder"]);
        // Everything after the choice is the same command.
        assert_eq!(named.args[2..], default.args[..]);
    }

    #[test]
    fn sec_18_the_linux_program_takes_the_name_of_the_release_from_its_folder() {
        for name in ["0.6.0", "0.6.0-test"] {
            let folder = Path::new("/mnt/c/work/target/release").join(name);
            assert_eq!(release_name_of(&folder, "0.6.0").unwrap(), name);
        }
        for other in ["0.5.1", "0.6.0-rc", ""] {
            let folder = Path::new("/mnt/c/work/target/release").join(other);
            assert!(release_name_of(&folder, "0.6.0").is_err(), "{other}");
        }
    }

    #[test]
    fn sec_18_the_release_refuses_files_of_another_version_or_none() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            embedded_problem(root.path(), "0.6.0")
                .unwrap()
                .contains("missing")
        );
        let folder = root.path().join(super::super::EMBEDDED_FOLDER);
        fs::create_dir_all(&folder).unwrap();
        let file = folder.join(about::embedded_name(about::CYCLONEDX_END));
        fs::write(&file, r#"{"metadata":{"component":{"version":"0.5.1"}}}"#).unwrap();
        assert!(
            embedded_problem(root.path(), "0.6.0")
                .unwrap()
                .contains("0.5.1")
        );
        fs::write(&file, r#"{"metadata":{"component":{"version":"0.6.0"}}}"#).unwrap();
        assert_eq!(embedded_problem(root.path(), "0.6.0"), None);
    }

    #[test]
    fn sec_18_the_notes_carry_the_keys_the_support_period_and_a_test_build_mark() {
        let root = super::super::repository_root();
        let untagged = Build {
            tagged: false,
            test_keys: false,
        };
        let text = notes(&root, "0.6.0", untagged).unwrap();
        for key in release::RELEASE_PUBLIC_KEYS {
            assert!(text.contains(key.public), "{}", key.name);
        }
        assert!(text.contains("test build"), "{text}");
        assert!(text.contains("0.6.0"));
        assert!(!text.contains("{version}"));
        let test_keys = Build {
            tagged: true,
            test_keys: true,
        };
        let text = notes(&root, "0.6.0", test_keys).unwrap();
        assert!(text.contains("signed with a test key"), "{text}");
        let real = Build {
            tagged: true,
            test_keys: false,
        };
        let text = notes(&root, "0.6.0", real).unwrap();
        assert!(
            text.contains("This is the release of the tag `v0.6.0`."),
            "{text}"
        );
    }

    #[test]
    fn sec_18_a_test_build_carries_test_in_its_one_release_name() {
        for (tagged, test_keys, name) in [
            (true, false, "0.6.0"),
            (false, false, "0.6.0-test"),
            (true, true, "0.6.0-test"),
            (false, true, "0.6.0-test"),
        ] {
            let build = Build { tagged, test_keys };
            assert_eq!(build.release_name("0.6.0"), name, "{build:?}");
            // The folder and every file carry the name, and so does every trusted comment.
            for file in folder::files(name) {
                assert!(
                    file.contains(name) || file.starts_with(release::CHECKSUMS_FILE),
                    "{file}"
                );
                assert!(
                    folder::trusted_comment(&file, name).ends_with(&format!("version:{name}")),
                    "{file}"
                );
            }
        }
    }

    #[test]
    fn sec_18_the_release_date_is_the_day_of_the_tags_commit_when_the_tag_exists() {
        let day = jiff::civil::date(2026, 12, 1);
        assert_eq!(date_problem("0.6.0", day, None), None);
        assert_eq!(date_problem("0.6.0", day, Some(day)), None);
        let problem = date_problem("0.6.0", day, Some(jiff::civil::date(2026, 12, 2))).unwrap();
        assert!(
            problem.contains("2026-12-01")
                && problem.contains("2026-12-02")
                && problem.contains("v0.6.0"),
            "{problem}"
        );
    }
}
