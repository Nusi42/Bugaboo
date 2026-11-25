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

/// The folders of this computer that the build of the workspace at `root` reads from, from cargo
/// (`cargo metadata`) and from rustc (`rustc --print sysroot`).
pub(crate) fn of_this_computer(root: &Path) -> anyhow::Result<Vec<Folder>> {
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
    Ok(renamed(step, &of_this_computer(root)?))
}

fn renamed(mut step: Step, folders: &[Folder]) -> Step {
    step.args
        .extend(["--config".to_owned(), cargo_setting(folders)]);
    step
}

/// `rustflags` (the text of `RUSTFLAGS`) with the folders of this computer renamed. `RUSTFLAGS`
/// splits at spaces, so a folder with a space in its path keeps its name, with a note: the build
/// works, and SEC-BUILD-PATHS refuses what it made.
pub(crate) fn with_rustflags(root: &Path, rustflags: &str) -> anyhow::Result<String> {
    let folders = of_this_computer(root)?;
    let (fit, unfit): (Vec<Folder>, Vec<Folder>) = folders
        .into_iter()
        .partition(|folder| !folder.path.contains(char::is_whitespace));
    for folder in &unfit {
        crate::output::note(&format!(
            "{} has a space in its path, which RUSTFLAGS cannot carry: the WebAssembly modules name \
             it, and a release refuses them",
            folder.what
        ));
    }
    let mut options = vec![rustflags.trim().to_owned()];
    options.extend(compiler_options(&fit));
    Ok(options.join(" ").trim().to_owned())
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
            assert!(Path::new(&folder.path).is_absolute(), "{}", folder.what);
            assert!(Path::new(&folder.path).is_dir(), "{}", folder.what);
        }
        let flags = with_rustflags(&root, " -Zfmt-debug=none ").unwrap();
        assert!(
            flags.starts_with("-Zfmt-debug=none --remap-path-prefix="),
            "{}",
            folders.len()
        );
    }
}
