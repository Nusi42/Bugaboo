//! Tools of `kind = "bundle"` (tools/dev/tools.toml): an archive with several folders, such as
//! PostgreSQL, unpacked into `.tools/<name>/`. A platform without a download entry uses the
//! program installed on the machine instead, and setup says how to install it when it is missing.

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};

use crate::output;
use crate::setup;
use crate::tools::{self, Tool};

/// Installs a bundle tool, or checks that the system provides it.
pub(crate) fn install(root: &Path, tool: &Tool) -> anyhow::Result<()> {
    install_with(root, tool, &setup::fetch)
}

pub(crate) fn install_with(
    root: &Path,
    tool: &Tool,
    get: &dyn Fn(&str, &Path) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let Some(download) = tool.download_if_any() else {
        return check_system(root, tool);
    };
    if tool.folders.is_empty() {
        bail!(
            "{}: kind `bundle` needs `folders` in {}.",
            tool.name,
            tools::TOOLS_PATH
        );
    }
    let target = root.join(tools::TOOLS_DIR).join(&tool.name);
    let marker = root
        .join(tools::TOOLS_DIR)
        .join("versions")
        .join(&tool.name);
    let complete = tool
        .folders
        .iter()
        .all(|folder| target.join(folder).is_dir());
    if complete && fs::read_to_string(&marker).is_ok_and(|v| v.trim() == tool.version) {
        output::info(&format!(
            "ok       {} {} (already installed)",
            tool.name, tool.version
        ));
        return Ok(());
    }
    output::info(&format!(
        "download {} {} (large; unpacking only {})",
        tool.name,
        tool.version,
        tool.folders.join(", ")
    ));
    let file_name = download.url.rsplit('/').next().unwrap_or("download");
    let cache = tools::cache_dir(root);
    fs::create_dir_all(&cache)?;
    let archive = cache.join(file_name);
    let bytes = cached_or_fetched(&archive, &download.url, &download.sha256, get)?;
    let staging = with_suffix(&target, ".new");
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    extract_folders(&bytes, &tool.folders, &staging)
        .with_context(|| format!("could not unpack {file_name}"))?;
    if target.exists() {
        fs::remove_dir_all(&target).with_context(|| {
            format!(
                "could not remove the old {}; stop programs that use it (`cargo xtask db stop`) \
                 and retry",
                target.display()
            )
        })?;
    }
    fs::rename(&staging, &target)?;
    if let Some(folder) = marker.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(&marker, &tool.version)?;
    Ok(())
}

/// The archive bytes: the cached file when its checksum is right, else a fresh download.
fn cached_or_fetched(
    archive: &Path,
    url: &str,
    sha256: &str,
    get: &dyn Fn(&str, &Path) -> anyhow::Result<()>,
) -> anyhow::Result<Vec<u8>> {
    if let Ok(bytes) = fs::read(archive)
        && setup::verify_sha256(&bytes, sha256, url).is_ok()
    {
        return Ok(bytes);
    }
    get(url, archive)?;
    let bytes = fs::read(archive)?;
    if let Err(error) = setup::verify_sha256(&bytes, sha256, url) {
        // A bad file must never be reused on the next run.
        let _ = fs::remove_file(archive);
        return Err(error);
    }
    Ok(bytes)
}

/// Unpacks the wanted top-level folders of a `.zip` (below its single root folder) into `to`.
pub(crate) fn extract_folders(bytes: &[u8], folders: &[String], to: &Path) -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let mut count = 0_usize;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let Some(path) = entry.enclosed_name() else {
            bail!("the archive contains the unsafe path {}", entry.name());
        };
        let mut parts = path.components();
        // The first component is the archive's root folder (`pgsql`).
        let (Some(_), Some(top)) = (parts.next(), parts.next()) else {
            continue;
        };
        if !folders
            .iter()
            .any(|folder| top.as_os_str() == folder.as_str())
        {
            continue;
        }
        let relative: PathBuf = path.components().skip(1).collect();
        let destination = to.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&destination)?;
            continue;
        }
        if let Some(folder) = destination.parent() {
            fs::create_dir_all(folder)?;
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        fs::write(&destination, data)?;
        count += 1;
    }
    if count == 0 {
        bail!("none of the folders {folders:?} were found in the archive");
    }
    Ok(())
}

fn check_system(root: &Path, tool: &Tool) -> anyhow::Result<()> {
    for program in &tool.system_programs {
        if find_program(root, tool, program).is_none() {
            bail!(
                "{} {} is not installed, and {} has no download for this system. {}",
                tool.name,
                tool.version,
                tools::TOOLS_PATH,
                tool.system_hint
                    .as_deref()
                    .unwrap_or("Install it with your package manager.")
            );
        }
    }
    output::info(&format!(
        "ok       {} (installed on this system)",
        tool.name
    ));
    Ok(())
}

/// Finds a program of a bundle tool: in the unpacked bundle, then on the PATH, then in the
/// folder that the system names for it.
pub(crate) fn find_program(root: &Path, tool: &Tool, program: &str) -> Option<PathBuf> {
    let file = tools::exe(program);
    let bundled = root
        .join(tools::TOOLS_DIR)
        .join(&tool.name)
        .join("bin")
        .join(&file);
    if bundled.is_file() {
        return Some(bundled);
    }
    find_on_path(program).or_else(|| {
        folder_named_by_system(tool, root)
            .map(|folder| folder.join(&file))
            .filter(|candidate| candidate.is_file())
    })
}

/// The folder of the programs that the system itself names: the output of the command the tool
/// lists (`pg_config --bindir`), when it is one existing folder. The system answers; no folder of
/// it is written down here (coding-standards §2, No fixed paths).
fn folder_named_by_system(tool: &Tool, root: &Path) -> Option<PathBuf> {
    let (program, arguments) = tool.system_folder_command.split_first()?;
    let program = find_on_path(program)?;
    let output = std::process::Command::new(program)
        .args(arguments)
        .current_dir(root)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| folder_in_answer(&String::from_utf8_lossy(&output.stdout)))
        .flatten()
}

/// The one existing, absolute folder in the text a command printed.
fn folder_in_answer(answer: &str) -> Option<PathBuf> {
    let mut lines = answer
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let folder = PathBuf::from(lines.next()?);
    (lines.next().is_none() && folder.is_absolute() && folder.is_dir()).then_some(folder)
}

/// The file `file` in the first folder of the search path that holds it.
fn find_on_path(file: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|folder| folder.join(tools::exe(file)))
        .find(|candidate| candidate.is_file())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;
    use crate::testutil::TempDir;

    fn zip_with(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
            let options = zip::write::SimpleFileOptions::default();
            for (name, data) in files {
                writer.start_file(*name, options).unwrap();
                writer.write_all(data).unwrap();
            }
            writer.finish().unwrap();
        }
        bytes
    }

    fn tool(extra: &str) -> Tool {
        let text = format!(
            "[[tool]]\nname = \"pg\"\nkind = \"bundle\"\nversion = \"1\"\nfolders = [\"bin\"]\n{extra}"
        );
        tools::parse(&text).unwrap().remove(0)
    }

    #[test]
    fn good_example_only_the_wanted_folders_are_unpacked_without_the_root() {
        let temp = TempDir::new("bundle-extract");
        let bytes = zip_with(&[
            ("pgsql/bin/pg_ctl.exe", b"x"),
            ("pgsql/lib/libpq.dll", b"y"),
            ("pgsql/pgAdmin/huge.bin", b"z"),
        ]);
        let wanted = vec!["bin".to_owned(), "lib".to_owned()];
        extract_folders(&bytes, &wanted, temp.path()).unwrap();
        assert!(temp.path().join("bin/pg_ctl.exe").is_file());
        assert!(temp.path().join("lib/libpq.dll").is_file());
        assert!(!temp.path().join("pgAdmin").exists());
    }

    #[test]
    fn bad_example_an_archive_without_the_folders_is_refused() {
        let temp = TempDir::new("bundle-empty");
        let bytes = zip_with(&[("pgsql/doc/readme.txt", b"x")]);
        assert!(extract_folders(&bytes, &["bin".to_owned()], temp.path()).is_err());
    }

    #[test]
    fn bad_example_a_wrong_checksum_installs_nothing() {
        let temp = TempDir::new("bundle-checksum");
        let entry = tool(&format!(
            "windows-x64 = {{ url = \"https://example.test/a.zip\", sha256 = \"{}\" }}\n\
             linux-x64 = {{ url = \"https://example.test/a.zip\", sha256 = \"{}\" }}\n",
            "0".repeat(64),
            "0".repeat(64)
        ));
        let get = |_: &str, to: &Path| {
            fs::write(to, zip_with(&[("pgsql/bin/x", b"x")]))?;
            Ok(())
        };
        let error = install_with(temp.path(), &entry, &get).unwrap_err();
        assert!(error.to_string().contains("Checksum mismatch"), "{error}");
        assert!(!temp.path().join(".tools/pg").exists());
    }

    #[test]
    fn missing_system_program_says_how_to_install_it() {
        let temp = TempDir::new("bundle-system");
        let entry = tool(
            "system-programs = [\"bb-no-such-program\"]\nsystem-hint = \"Install it somehow.\"\n",
        );
        let error = install_with(temp.path(), &entry, &|_, _| Ok(())).unwrap_err();
        assert!(error.to_string().contains("Install it somehow."), "{error}");
    }

    #[test]
    fn the_folder_the_system_names_is_one_existing_absolute_folder() {
        let temp = TempDir::new("bundle-named");
        let said = format!(
            "  {}
",
            temp.path().display()
        );
        assert_eq!(folder_in_answer(&said), Some(temp.path().to_path_buf()));
        assert_eq!(folder_in_answer(""), None);
        assert_eq!(
            folder_in_answer("bin"),
            None,
            "a relative folder is no answer"
        );
        let missing = temp.path().join("not-there");
        assert_eq!(folder_in_answer(&missing.display().to_string()), None);
        let two = format!(
            "{0}
{0}
",
            temp.path().display()
        );
        assert_eq!(folder_in_answer(&two), None, "two lines are no answer");
    }

    #[test]
    fn a_tool_without_a_command_names_no_folder() {
        let temp = TempDir::new("bundle-no-command");
        let entry = tool(
            "system-programs = [\"bb-no-such-program\"]
",
        );
        assert_eq!(folder_named_by_system(&entry, temp.path()), None);
    }
}
