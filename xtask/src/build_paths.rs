//! The folders of the computer that a build reads its sources from, and the neutral names that the
//! files which ship give them instead (coding-standards §2, *No folder of the building computer in
//! what ships*; SEC-BUILD-PATHS).
//!
//! The compiler writes the path of a source file into a program wherever a panic can name its
//! place. Cargo hands it the sources of the dependencies (in the home of cargo), of the standard
//! library when it is built again (in the toolchain) and of generated code (in the build folder) by
//! their absolute paths, and those name the account of the computer. Every build that ships asks
//! the compiler for neutral names instead (`--remap-path-prefix`): the release programs, on Windows
//! and in WSL alike, and the WebAssembly modules the programs carry. The folders are asked of cargo
//! and of rustc each time, never written down. The standard library that comes with the toolchain
//! already names its sources `/rustc/<commit>/…`, which says nothing about the computer.

use std::path::Path;

use anyhow::{Context as _, bail};

use crate::steps::Step;

/// A folder of this computer that a build reads from, and the name the files that ship give it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Folder {
    /// The path as cargo and rustc name it.
    pub(crate) path: String,
    /// The name the compiler writes in its place.
    pub(crate) neutral: &'static str,
    /// What the folder is, for a finding (never the path, which names the account).
    pub(crate) what: &'static str,
}

/// The neutral names, one for each folder.
const REPOSITORY: &str = "/bugaboo";
const BUILD_FOLDER: &str = "/bugaboo-build";
const CARGO_HOME: &str = "/cargo";
const TOOLCHAIN: &str = "/rustup/toolchain";

/// The variables that hand the compiler its options, which replace the ones the settings files
/// name; the second splits them at `\x1f` instead of at spaces.
pub(crate) const RUSTFLAGS: &str = "RUSTFLAGS";
pub(crate) const ENCODED_RUSTFLAGS: &str = "CARGO_ENCODED_RUSTFLAGS";
/// What separates the options in [`ENCODED_RUSTFLAGS`].
const ENCODED_SEPARATOR: char = '\x1f';

/// The folders of this computer that the build of the workspace at `root` reads from, from cargo
/// (`cargo metadata`) and from rustc (`rustc --print sysroot`). They are asked once per command:
/// one run of xtask builds one repository.
pub(crate) fn of_this_computer(root: &Path) -> anyhow::Result<&'static [Folder]> {
    static FOLDERS: std::sync::OnceLock<(std::path::PathBuf, Vec<Folder>)> =
        std::sync::OnceLock::new();
    if let Some((asked_for, folders)) = FOLDERS.get() {
        debug_assert_eq!(
            asked_for, root,
            "the folders are asked once per command, for one repository"
        );
        return Ok(folders);
    }
    let found = asked(root)?;
    Ok(&FOLDERS.get_or_init(|| (root.to_path_buf(), found)).1)
}

fn asked(root: &Path) -> anyhow::Result<Vec<Folder>> {
    let cargo = crate::metadata::build_folders(root)?;
    let mut folders = vec![
        folder(&cargo.workspace, REPOSITORY, "the folder of the repository")?,
        folder(&cargo.target, BUILD_FOLDER, "the build folder")?,
        folder(
            Path::new(&sysroot(root)?),
            TOOLCHAIN,
            "the folder of the Rust toolchain",
        )?,
    ];
    if let Some(home) = &cargo.cargo_home {
        folders.push(folder(home, CARGO_HOME, "the home of cargo")?);
    }
    Ok(arranged(folders))
}

fn folder(path: &Path, neutral: &'static str, what: &'static str) -> anyhow::Result<Folder> {
    let Some(text) = path.to_str() else {
        bail!("{what} has a name that is not UTF-8, which the compiler cannot be told to rename");
    };
    Ok(Folder {
        path: text.to_owned(),
        neutral,
        what,
    })
}

/// The folder of the toolchain this repository builds with (`rust-toolchain.toml`).
fn sysroot(root: &Path) -> anyhow::Result<String> {
    let out = crate::tools::command("rustc")
        .args(["--print", "sysroot"])
        .current_dir(root)
        .output()
        .context("could not run rustc; is Rust installed?")?;
    if !out.status.success() {
        bail!(
            "rustc did not name its folder: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The folders, each once, the shortest first: the compiler takes the last name that fits, so a
/// folder inside another (the build folder in the repository) gets its own.
fn arranged(mut folders: Vec<Folder>) -> Vec<Folder> {
    folders.retain(|folder| !folder.path.is_empty());
    folders.sort_by_key(|folder| folder.path.len());
    folders.dedup_by(|later, earlier| later.path == earlier.path);
    folders
}

/// What the compiler is told for the folders.
pub(crate) fn compiler_options(folders: &[Folder]) -> Vec<String> {
    folders
        .iter()
        .map(|folder| format!("--remap-path-prefix={}={}", folder.path, folder.neutral))
        .collect()
}

/// The setting of cargo that hands the compiler the options for every target, added to the ones
/// the settings files name (the C runtime of Windows): `--config` joins its list with theirs,
/// where `RUSTFLAGS` would replace them.
fn cargo_setting(folders: &[Folder]) -> String {
    let options: Vec<String> = compiler_options(folders)
        .iter()
        .map(|option| serde_json::Value::from(option.as_str()).to_string())
        .collect();
    format!("target.'cfg(all())'.rustflags = [{}]", options.join(", "))
}

/// The optimised cargo build `step` of the workspace at `root` with the folders of this computer
/// renamed in what it makes. Every optimised build of a program asks for it, the ones that do not
/// ship too, so that they share what cargo compiled.
pub(crate) fn neutral(step: Step, root: &Path) -> anyhow::Result<Step> {
    for variable in [RUSTFLAGS, ENCODED_RUSTFLAGS] {
        if std::env::var_os(variable).is_some() {
            crate::output::note(&format!(
                "{variable} is set: cargo then ignores the options of the settings files, so this \
                 build names the folders of this computer and leaves out the C runtime of \
                 Windows; unset it"
            ));
        }
    }
    Ok(renamed(step, of_this_computer(root)?))
}

fn renamed(mut step: Step, folders: &[Folder]) -> Step {
    step.args
        .extend(["--config".to_owned(), cargo_setting(folders)]);
    step
}

/// The value of [`ENCODED_RUSTFLAGS`] for a build that sets its own options (`extra`): the options
/// of the environment (that variable, else `RUSTFLAGS` split at spaces), the folders of this
/// computer renamed, then `extra`. A folder with a space in its path stays one option.
pub(crate) fn encoded_rustflags(root: &Path, extra: &[&str]) -> anyhow::Result<String> {
    Ok(encoded(
        std::env::var(ENCODED_RUSTFLAGS).ok().as_deref(),
        std::env::var(RUSTFLAGS).ok().as_deref(),
        of_this_computer(root)?,
        extra,
    ))
}

fn encoded(
    encoded: Option<&str>,
    plain: Option<&str>,
    folders: &[Folder],
    extra: &[&str],
) -> String {
    let mut options: Vec<String> = match (encoded, plain) {
        (Some(encoded), _) if !encoded.is_empty() => encoded
            .split(ENCODED_SEPARATOR)
            .map(str::to_owned)
            .collect(),
        (_, Some(plain)) => plain.split_whitespace().map(str::to_owned).collect(),
        _ => Vec::new(),
    };
    options.extend(compiler_options(folders));
    options.extend(extra.iter().map(|option| (*option).to_owned()));
    options.join(&ENCODED_SEPARATOR.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(path: &str, neutral: &'static str) -> Folder {
        Folder {
            path: path.to_owned(),
            neutral,
            what: "a folder",
        }
    }

    #[test]
    fn sec_18_a_folder_inside_another_is_renamed_after_it() {
        let folders = arranged(vec![
            named(r"C:\Users\someone\.cargo", CARGO_HOME),
            named(r"C:\work\bugaboo\target", BUILD_FOLDER),
            named(r"C:\work\bugaboo", REPOSITORY),
            named(r"C:\work\bugaboo", REPOSITORY),
            named("", TOOLCHAIN),
        ]);
        assert_eq!(
            compiler_options(&folders),
            [
                r"--remap-path-prefix=C:\work\bugaboo=/bugaboo",
                r"--remap-path-prefix=C:\work\bugaboo\target=/bugaboo-build",
                r"--remap-path-prefix=C:\Users\someone\.cargo=/cargo",
            ]
        );
    }

    #[test]
    fn sec_18_a_release_build_joins_the_renaming_to_the_settings_files() {
        let step = Step::new("build", "cargo", &["build", "--release"]);
        let folders = [named(r"C:\Users\someone\a b\.cargo", CARGO_HOME)];
        let step = renamed(step, &folders);
        assert_eq!(step.args[..3], ["build", "--release", "--config"]);
        // A TOML list of strings, the backslashes and the space kept.
        assert_eq!(
            step.args[3],
            r#"target.'cfg(all())'.rustflags = ["--remap-path-prefix=C:\\Users\\someone\\a b\\.cargo=/cargo"]"#
        );
    }

    #[test]
    fn sec_18_the_folders_of_this_computer_come_from_cargo_and_rustc() {
        let root = crate::release::repository_root();
        let folders = of_this_computer(&root).unwrap();
        for neutral in [REPOSITORY, BUILD_FOLDER, TOOLCHAIN, CARGO_HOME] {
            let folder = folders
                .iter()
                .find(|folder| folder.neutral == neutral)
                .unwrap_or_else(|| panic!("no folder for {neutral}"));
            assert!(
                Path::new(&folder.path).is_absolute(),
                "cargo or rustc named {} by a relative path",
                folder.what
            );
            assert!(
                Path::new(&folder.path).is_dir(),
                "{} is not a folder on this computer",
                folder.what
            );
        }
        let flags = encoded_rustflags(&root, &["-Zfmt-debug=none"]).unwrap();
        assert!(
            flags.matches("--remap-path-prefix=").count() == folders.len()
                && flags.ends_with("\x1f-Zfmt-debug=none"),
            "the module is built with every folder renamed ({} of them) and its own options last",
            folders.len()
        );
    }

    #[test]
    fn sec_18_the_options_of_a_module_keep_a_folder_with_a_space_whole() {
        let folders = [named(r"C:\Users\someone\a b\.cargo", CARGO_HOME)];
        assert_eq!(
            encoded(None, Some(" -Ca=1  -Cb=2 "), &folders, &["-Zx"]),
            "-Ca=1\x1f-Cb=2\x1f--remap-path-prefix=C:\\Users\\someone\\a b\\.cargo=/cargo\x1f-Zx"
        );
        // The encoded options of the environment win over RUSTFLAGS, as they do for cargo.
        let from_encoded = encoded(Some("-Ca=1 x\x1f-Cb=2"), Some("-Cignored"), &folders, &[]);
        assert!(
            from_encoded.starts_with("-Ca=1 x\x1f-Cb=2\x1f--remap")
                && !from_encoded.contains("ignored"),
            "{from_encoded:?}"
        );
        assert_eq!(encoded(None, None, &[], &["-Zx"]), "-Zx");
    }
}
