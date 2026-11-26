//! SEC-BUILD-PATHS: the files that ship name no folder of the computer that built them
//! (coding-standards §2, *No folder of the building computer in what ships*).
//!
//! A file is read as bytes and searched for the start of a profile or home folder (a drive with
//! `Users`, and `/home/`, `/Users/` and `/root/`, which also find `/mnt/c/Users/…` of WSL) and for
//! the folders of this computer that a build reads from ([`build_paths::of_this_computer`]), with
//! either separator; a backslash may be doubled, as JSON and JavaScript escape it, and a drive
//! with `Users` is also found in UTF-16 and as WSL names it (`/mnt/c/users/…` in any case). The
//! release command searches its programs before it copies them into the release folder, the whole
//! folder when it is written and again before it signs it, so a file that names a folder is never
//! signed (`release::refuse_build_paths`); `release check` searches the folder too. In the full
//! run the check searches what the browser loads: the WebAssembly modules and their glue, and the
//! built browser app when it is there. The toolchain's own standard library names its sources
//! `/rustc/<commit>/…`, which says nothing about the computer and is not searched for. A finding
//! names the kind of folder and how often it was found, never the text found, which holds the
//! account.

use std::path::{Path, PathBuf};

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::build_paths::{self, Folder};
use crate::steps::{WEB_APP, WEB_DIST};
use crate::wasm;

/// The folders right under the root of a Unix system or of macOS that hold home folders, or are
/// one (the home of `root`).
const HOME_FOLDERS: [&str; 3] = ["home", "Users", "root"];
/// The folder of the profiles on a drive of Windows, after `<drive>:\`; any case.
const PROFILES_ON_A_DRIVE: &str = "users";
/// The folder under the root of WSL that holds the drives of Windows, one folder per letter.
const WSL_DRIVES: &str = "mnt";

pub(crate) struct SecBuildPaths;

impl Check for SecBuildPaths {
    fn id(&self) -> &'static str {
        "SEC-BUILD-PATHS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        wasm::ensure(root)?;
        let mut files = Vec::new();
        for module in wasm::MODULES {
            files.extend(shipped_in(&module.dir(root))?);
        }
        files.extend(shipped_in(&root.join(WEB_APP).join(WEB_DIST))?);
        let marks = marks(build_paths::of_this_computer(root)?);
        let mut findings = Vec::new();
        for file in files {
            if let Some(problem) = problem(&file, &marks)? {
                let place = file.strip_prefix(root).unwrap_or(&file);
                findings.push(Finding::at(place.to_string_lossy(), problem));
            }
        }
        Ok(findings)
    }
}

/// The files below `folder` that ship: all but the notes the build keeps for itself (their names
/// start with a dot, like the `.built` of a module, which names the build folder). None when the
/// folder is not there.
fn shipped_in(folder: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Ok(files);
    };
    for entry in entries {
        let path = entry
            .with_context(|| format!("could not read {}", folder.display()))?
            .path();
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('.'))
        {
            continue;
        }
        if path.is_dir() {
            files.extend(shipped_in(&path)?);
        } else {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// One kind of folder a file that ships may not name.
pub(crate) struct Mark {
    /// What it is, for the finding.
    what: String,
    test: Test,
}

enum Test {
    /// These bytes.
    Exact(Vec<u8>),
    /// `/<name>/`, also with the separator after it doubled.
    HomeFolder(&'static str),
    /// `<letter>:\Users\` with either separator, single or doubled (escaped in JSON and
    /// JavaScript), in any case.
    ProfilesOnADrive,
    /// The same in UTF-16 (little-endian), as Windows keeps some texts.
    WideProfilesOnADrive,
    /// `/mnt/<letter>/users/` of WSL in any case.
    MountedProfiles,
}

impl Test {
    /// The byte every match has at its start (for a drive, the colon after the letter).
    fn anchor(&self) -> u8 {
        match self {
            Self::Exact(bytes) => bytes[0],
            Self::HomeFolder(_) | Self::MountedProfiles => b'/',
            Self::ProfilesOnADrive | Self::WideProfilesOnADrive => b':',
        }
    }

    /// Whether a match starts at `at`, where the anchor is.
    fn matches_at(&self, bytes: &[u8], at: usize) -> bool {
        let rest = &bytes[at..];
        match self {
            Self::Exact(exact) => rest.starts_with(exact),
            Self::HomeFolder(name) => rest
                .get(1..)
                .and_then(|after| after.strip_prefix(name.as_bytes()))
                .is_some_and(|after| separators(after) > 0),
            Self::ProfilesOnADrive => {
                at.checked_sub(1)
                    .is_some_and(|letter| bytes[letter].is_ascii_alphabetic())
                    && profiles_after_colon(rest)
            }
            Self::WideProfilesOnADrive => {
                let letter = at.checked_sub(2).is_some_and(|letter| {
                    bytes[letter].is_ascii_alphabetic() && bytes[letter + 1] == 0
                });
                letter && profiles_after_colon(&narrow(rest))
            }
            Self::MountedProfiles => rest
                .get(1..)
                .and_then(|after| after.strip_prefix(WSL_DRIVES.as_bytes()))
                .and_then(|after| after.strip_prefix(b"/"))
                .filter(|after| {
                    after.len() > 2 && after[0].is_ascii_alphabetic() && after[1] == b'/'
                })
                .and_then(|after| after.get(2..2 + PROFILES_ON_A_DRIVE.len() + 1))
                .is_some_and(|folder| {
                    folder[..PROFILES_ON_A_DRIVE.len()]
                        .eq_ignore_ascii_case(PROFILES_ON_A_DRIVE.as_bytes())
                        && folder[PROFILES_ON_A_DRIVE.len()] == b'/'
                }),
        }
    }
}

/// How many separators `bytes` starts with, when they are one or two (a doubled backslash is an
/// escaped one); 0 otherwise.
fn separators(bytes: &[u8]) -> usize {
    let run = bytes
        .iter()
        .take(3)
        .take_while(|byte| is_separator(**byte))
        .count();
    if run <= 2 { run } else { 0 }
}

/// Whether `rest`, which starts at the colon after a drive letter, goes on with `\Users\`.
fn profiles_after_colon(rest: &[u8]) -> bool {
    let Some(after) = rest.get(1..) else {
        return false;
    };
    let before = separators(after);
    if before == 0 {
        return false;
    }
    let folder = &after[before..];
    folder.len() > PROFILES_ON_A_DRIVE.len()
        && folder[..PROFILES_ON_A_DRIVE.len()].eq_ignore_ascii_case(PROFILES_ON_A_DRIVE.as_bytes())
        && separators(&folder[PROFILES_ON_A_DRIVE.len()..]) > 0
}

/// The start of UTF-16 (little-endian) text as one byte per character, up to the first character
/// that is not ASCII.
fn narrow(wide: &[u8]) -> Vec<u8> {
    wide.chunks_exact(2)
        .take(PROFILES_ON_A_DRIVE.len() + 6)
        .take_while(|unit| unit[1] == 0 && unit[0].is_ascii())
        .map(|unit| unit[0])
        .collect()
}

fn is_separator(byte: u8) -> bool {
    matches!(byte, b'/' | b'\\')
}

/// The kinds of folders no file that ships may name: the profile and home folders of every
/// system, and the `folders` of this computer, with either separator and with the backslash
/// escaped.
pub(crate) fn marks(folders: &[Folder]) -> Vec<Mark> {
    let mut marks = vec![
        Mark {
            what: "a profile folder on a drive of Windows".to_owned(),
            test: Test::ProfilesOnADrive,
        },
        Mark {
            what: "a profile folder on a drive of Windows, in UTF-16".to_owned(),
            test: Test::WideProfilesOnADrive,
        },
        Mark {
            what: "a profile folder on a drive of Windows, as WSL names it".to_owned(),
            test: Test::MountedProfiles,
        },
    ];
    marks.extend(HOME_FOLDERS.map(|name| Mark {
        what: format!("the folder /{name}"),
        test: Test::HomeFolder(name),
    }));
    for folder in folders {
        let backslashed = folder.path.replace('/', "\\");
        let mut forms = vec![
            folder.path.clone(),
            folder.path.replace('\\', "/"),
            backslashed.replace('\\', "\\\\"),
            backslashed,
        ];
        forms.sort();
        forms.dedup();
        marks.extend(
            forms
                .into_iter()
                .filter(|form| !form.is_empty())
                .map(|form| Mark {
                    what: format!("{} of this computer", folder.what),
                    test: Test::Exact(form.into_bytes()),
                }),
        );
    }
    marks
}

/// How often each kind of folder is named in `bytes`, in the order of `marks`.
fn counts(bytes: &[u8], marks: &[Mark]) -> Vec<usize> {
    let mut by_anchor: [Vec<usize>; 256] = std::array::from_fn(|_| Vec::new());
    for (index, mark) in marks.iter().enumerate() {
        by_anchor[usize::from(mark.test.anchor())].push(index);
    }
    let mut counts = vec![0; marks.len()];
    for (at, byte) in bytes.iter().enumerate() {
        for &index in &by_anchor[usize::from(*byte)] {
            if marks[index].test.matches_at(bytes, at) {
                counts[index] += 1;
            }
        }
    }
    counts
}

/// What is wrong with the file at `path`, if it names a folder of `marks`: the kinds and how often,
/// never the text found.
pub(crate) fn problem(path: &Path, marks: &[Mark]) -> anyhow::Result<Option<String>> {
    let bytes =
        std::fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
    Ok(problem_in(&bytes, marks))
}

fn problem_in(bytes: &[u8], marks: &[Mark]) -> Option<String> {
    let named: Vec<String> = marks
        .iter()
        .zip(counts(bytes, marks))
        .filter(|(_, count)| *count > 0)
        .map(|(mark, count)| match count {
            1 => format!("{} (once)", mark.what),
            _ => format!("{} ({count} times)", mark.what),
        })
        .collect();
    (!named.is_empty()).then(|| {
        format!(
            "names folders of a computer: {}. A file that ships names none (coding-standards §2): \
             build it with `cargo xtask release`, which gives the folders neutral names.",
            named.join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn this_computer() -> Vec<Mark> {
        marks(&[
            Folder {
                path: r"D:\work\bugaboo".to_owned(),
                neutral: "/bugaboo",
                what: "the folder of the repository",
            },
            Folder {
                path: "/opt/cargo".to_owned(),
                neutral: "/cargo",
                what: "the home of cargo",
            },
        ])
    }

    #[test]
    fn bad_example_a_program_with_the_place_of_a_panic_is_refused() {
        let marks = this_computer();
        for planted in [
            r"C:\Users\someone\.cargo\registry\src\index\a-1.0.0\src\lib.rs",
            "c:/users/someone/.rustup/toolchains/a/lib/rustlib/src/rust/library/std/src/lib.rs",
            "/home/someone/.cargo/registry/src/a/lib.rs",
            "/root/.cargo/registry/src/index/a-1.0.0/src/lib.rs",
            "/mnt/c/Users/someone/work/bugaboo/crates/a/src/lib.rs",
            "/Users/someone/.cargo/registry/src/a/lib.rs",
            r"D:\work\bugaboo\target\release\build\a\out\generated.rs",
            "D:/work/bugaboo/crates/a/src/lib.rs",
            r"\opt\cargo\git\checkouts\a\src\lib.rs",
            // Escaped in JSON or JavaScript.
            r#"{"file":"C:\\Users\\someone\\.cargo\\registry\\a.rs"}"#,
            r#"{"file":"D:\\work\\bugaboo\\crates\\a\\src\\lib.rs"}"#,
            "/mnt/c/users/someone/work/bugaboo/crates/a/src/lib.rs",
        ] {
            let program = [
                b"\0\x7fELF called `Option::unwrap()`\0".as_slice(),
                planted.as_bytes(),
                b"\0",
            ]
            .concat();
            let problem = problem_in(&program, &marks).unwrap_or_else(|| panic!("{planted}"));
            assert!(!problem.contains("someone"), "{problem}");
            assert!(problem.contains("(once)"), "{problem}");
        }
        let wide: Vec<u8> = r"C:\Users\someone\.cargo\registry\a.rs"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let problem = problem_in(&wide, &marks).unwrap();
        assert!(problem.contains("in UTF-16 (once)"), "{problem}");
        let twice = b"/root/.cargo/a.rs\0/root/.cargo/b.rs";
        assert!(
            problem_in(twice, &marks)
                .unwrap()
                .contains("the folder /root (2 times)")
        );
    }

    #[test]
    fn good_example_a_program_with_neutral_names_passes() {
        let marks = this_computer();
        let program = b"/cargo/registry/src/index/a-1.0.0/src/lib.rs\0\
            /rustup/toolchain/lib/rustlib/src/rust/library/std/src/lib.rs\0\
            /rustc/0123456789abcdef0123456789abcdef01234567/library/core/src/panic.rs\0\
            /bugaboo/crates/a/src/lib.rs\0crates/a/src/lib.rs\0\
            https://github.com/someone/Bugaboo\0users: 3\0a:b\0/homepage/\0D:\\other\0\
            /api/users/1\0/mnt/data/users/\0";
        assert_eq!(problem_in(program, &marks), None);
        assert_eq!(problem_in(b"", &marks), None);
        assert_eq!(problem_in(b"C:", &marks), None);
        // A colon after a letter that looks like UTF-16 but goes on as one byte per character.
        assert_eq!(problem_in(b"A\0:", &marks), None);
        assert_eq!(problem_in(b"A\0:x", &marks), None);
    }

    #[test]
    fn good_example_the_notes_of_the_build_are_not_searched() {
        let folder = tempfile::tempdir().unwrap();
        let module = folder.path().join("module");
        std::fs::create_dir_all(module.join("deep")).unwrap();
        std::fs::write(module.join(".built"), "the build folder").unwrap();
        std::fs::write(module.join("a_bg.wasm"), "module").unwrap();
        std::fs::write(module.join("deep").join("a.js"), "glue").unwrap();
        assert_eq!(
            shipped_in(&module).unwrap(),
            [module.join("a_bg.wasm"), module.join("deep").join("a.js")]
        );
        assert!(
            shipped_in(&folder.path().join("missing"))
                .unwrap()
                .is_empty()
        );
    }
}
