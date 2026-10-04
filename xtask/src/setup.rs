//! `cargo xtask setup`: installs every pinned tool into `.tools/` (ADR-0014, ADR-0024).
//!
//! Nothing is installed machine-wide. Running it again changes nothing, and a download whose
//! sha256 does not match is refused before anything is unpacked.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, bail};
use sha2::{Digest as _, Sha256};

use crate::bundle;
use crate::output;
use crate::repo::Repo;
use crate::tools::{self, Tool};

/// Installs or updates every tool in `tools/dev/tools.toml`, then the npm packages.
pub(crate) fn run(repo: &Repo) -> anyhow::Result<()> {
    let list = tools::load(repo)?;
    let root = repo.root();
    fs::create_dir_all(tools::bin_dir(root))?;
    clean_old_binaries(&tools::bin_dir(root));
    let installed = installed_cargo_tools(root)?;
    for tool in &list {
        match tool.kind.as_str() {
            "cargo" => install_cargo(root, tool, &installed)?,
            "download" => install_download(root, tool)?,
            "bundle" => bundle::install(root, tool)?,
            other => bail!(
                "{}: unknown kind `{other}` in {}; use `cargo`, `download` or `bundle`.",
                tool.name,
                tools::TOOLS_PATH
            ),
        }
    }
    install_npm(root)
}

/// The crate names and versions already in `.tools`, from `cargo install --list`.
fn installed_cargo_tools(root: &Path) -> anyhow::Result<HashMap<String, String>> {
    let out = Command::new("cargo")
        .args(["install", "--list", "--root", tools::TOOLS_DIR])
        .current_dir(root)
        .output()
        .context("could not run cargo; is Rust installed?")?;
    Ok(parse_install_list(&String::from_utf8_lossy(&out.stdout)))
}

/// Lines such as `taplo-cli v0.10.0:` start a crate; indented lines are its programs.
fn parse_install_list(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter(|line| !line.starts_with(char::is_whitespace))
        .filter_map(|line| {
            let (name, rest) = line.split_once(" v")?;
            let version = rest.trim_end_matches(':').split_whitespace().next()?;
            Some((name.to_owned(), version.to_owned()))
        })
        .collect()
}

fn install_cargo(
    root: &Path,
    tool: &Tool,
    installed: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let crate_name = tool
        .crate_name
        .as_deref()
        .with_context(|| format!("{}: kind `cargo` needs a `crate` name", tool.name))?;
    let binary = tools::bin_dir(root).join(tools::exe(&tool.name));
    if installed.get(crate_name) == Some(&tool.version) && binary.is_file() {
        output::info(&format!(
            "ok       {} {} (already installed)",
            tool.name, tool.version
        ));
        return Ok(());
    }
    output::info(&format!(
        "install  {} {} (cargo install; this can take a few minutes)",
        tool.name, tool.version
    ));
    let status = Command::new("cargo")
        .args([
            "install",
            "--locked",
            "--root",
            tools::TOOLS_DIR,
            crate_name,
            "--version",
        ])
        .arg(&tool.version)
        .current_dir(root)
        .status()
        .context("could not run cargo")?;
    if !status.success() {
        bail!(
            "cargo install of {crate_name} {} failed (see the messages above). On Windows this \
             also happens while the tool is running: close {} and run `cargo xtask setup` again.",
            tool.version,
            binary.display()
        );
    }
    Ok(())
}

fn install_download(root: &Path, tool: &Tool) -> anyhow::Result<()> {
    install_download_with(root, tool, &fetch)
}

/// Downloads with `get` (curl in real runs, a local copy in the tests).
fn install_download_with(
    root: &Path,
    tool: &Tool,
    get: &dyn Fn(&str, &Path) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let binary = if cfg!(windows) {
        format!("{}.exe", tool.name)
    } else {
        tool.name.clone()
    };
    let target = tools::bin_dir(root).join(&binary);
    let marker = root
        .join(tools::TOOLS_DIR)
        .join("versions")
        .join(&tool.name);
    if target.is_file() && fs::read_to_string(&marker).is_ok_and(|v| v.trim() == tool.version) {
        output::info(&format!(
            "ok       {} {} (already installed)",
            tool.name, tool.version
        ));
        return Ok(());
    }
    let download = tool.download()?;
    output::info(&format!("download {} {}", tool.name, tool.version));
    let file_name = download.url.rsplit('/').next().unwrap_or("download");
    let cache = root.join(tools::TOOLS_DIR).join("cache");
    fs::create_dir_all(&cache)?;
    let archive = cache.join(file_name);
    get(&download.url, &archive)?;
    let bytes = fs::read(&archive)?;
    if let Err(error) = verify_sha256(&bytes, &download.sha256, &download.url) {
        // A bad file must never be reused on the next run.
        let _ = fs::remove_file(&archive);
        return Err(error);
    }
    let contents = extract_binary(file_name, &bytes, &binary)?;
    place_binary(&target, &contents)?;
    if let Some(folder) = marker.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(&marker, &tool.version)?;
    Ok(())
}

/// Downloads `url` to `destination` with curl (present on every supported system).
pub(crate) fn fetch(url: &str, destination: &Path) -> anyhow::Result<()> {
    let status = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--retry",
            "2",
            "--output",
        ])
        .arg(destination)
        .arg(url)
        .status()
        .context("could not run curl; it is needed to download tools")?;
    if !status.success() {
        bail!("Downloading {url} failed (see the message above). Check your internet connection.");
    }
    Ok(())
}

/// Stops with a clear message unless `bytes` has exactly the expected sha256.
pub(crate) fn verify_sha256(bytes: &[u8], expected: &str, source: &str) -> anyhow::Result<()> {
    let digest = Sha256::digest(bytes);
    let mut actual = String::new();
    for byte in &digest {
        let _ = write!(actual, "{byte:02x}");
    }
    if actual.eq_ignore_ascii_case(expected.trim()) {
        return Ok(());
    }
    bail!(
        "Checksum mismatch for {source}: expected sha256 {expected} but got {actual}. \
         Nothing was installed. If the tool was updated on purpose, fix the sha256 in {}.",
        tools::TOOLS_PATH
    )
}

/// Returns the bytes of the file named `binary` inside a `.zip` or `.tar.gz` archive.
pub(crate) fn extract_binary(
    archive_name: &str,
    bytes: &[u8],
    binary: &str,
) -> anyhow::Result<Vec<u8>> {
    let mut found = None;
    if Path::new(archive_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index)?;
            if entry.is_file() && file_name_of(entry.name()) == binary {
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                found = Some(data);
                break;
            }
        }
    } else if archive_name.ends_with(".tar.gz") {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
        for entry in tar.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_string_lossy().into_owned();
            if entry.header().entry_type().is_file() && file_name_of(&path) == binary {
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                found = Some(data);
                break;
            }
        }
    } else {
        bail!("{archive_name}: only .zip and .tar.gz archives are supported.");
    }
    found.with_context(|| format!("{archive_name} does not contain `{binary}`."))
}

fn file_name_of(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Puts `contents` at `target` even while an older copy is running (Windows locks running
/// programs but allows renaming them): write `.new`, move the old file to `.old`, rename `.new`.
fn place_binary(target: &Path, contents: &[u8]) -> anyhow::Result<()> {
    if let Some(folder) = target.parent() {
        fs::create_dir_all(folder)?;
    }
    let new = with_suffix(target, ".new");
    let old = with_suffix(target, ".old");
    fs::write(&new, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&new, fs::Permissions::from_mode(0o755))?;
    }
    let _ = fs::remove_file(&old);
    if target.exists() {
        fs::rename(target, &old).with_context(|| {
            format!(
                "could not move the old {} aside; close it and retry",
                target.display()
            )
        })?;
    }
    fs::rename(&new, target)?;
    Ok(())
}

fn with_suffix(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

/// Removes the `.old` copies left by an earlier update (they were still running then).
fn clean_old_binaries(bin: &Path) {
    let Ok(entries) = fs::read_dir(bin) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.path().extension().is_some_and(|ext| ext == "old") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// `npm ci` when the repository has a root `package.json` and `node_modules` is out of date.
///
/// `--ignore-scripts`: no package runs install scripts on this machine. None needs them today;
/// a package that truly does must be listed here with the reason.
fn install_npm(root: &Path) -> anyhow::Result<()> {
    if !root.join("package.json").is_file() {
        return Ok(());
    }
    if npm_is_current(root) {
        output::info("ok       npm packages (already installed)");
        return Ok(());
    }
    output::info("install  npm packages (npm ci)");
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(npm)
        .args(["ci", "--ignore-scripts"])
        .current_dir(root)
        .status()
        .context("could not run npm; is Node.js installed?")?;
    if !status.success() {
        bail!("npm ci failed (see the messages above).");
    }
    Ok(())
}

/// Whether npm's own record of what is installed (`node_modules/.package-lock.json`) has every
/// package of `package-lock.json` at the same version and checksum.
fn npm_is_current(root: &Path) -> bool {
    let read = |path: std::path::PathBuf| {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
    };
    match (
        read(root.join("package-lock.json")),
        read(root.join("node_modules").join(".package-lock.json")),
    ) {
        (Some(lock), Some(installed)) => lock_is_installed(&lock, &installed),
        _ => false,
    }
}

fn lock_is_installed(lock: &serde_json::Value, installed: &serde_json::Value) -> bool {
    let Some(wanted) = lock.get("packages").and_then(serde_json::Value::as_object) else {
        return false;
    };
    let have = installed.get("packages");
    wanted
        .iter()
        .filter(|(key, entry)| {
            // Workspace folders (`web/app`) are not in npm's hidden lockfile (M0-F12); only what
            // lives below `node_modules/` is compared.
            key.starts_with("node_modules/")
                && entry.get("link").and_then(serde_json::Value::as_bool) != Some(true)
        })
        .all(|(key, entry)| {
            have.and_then(|have| have.get(key)).is_some_and(|there| {
                there.get("version") == entry.get("version")
                    && there.get("integrity") == entry.get("integrity")
            })
        })
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;
    use crate::testutil::TempDir;

    const SHA_OF_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn good_example_matching_checksum_passes() {
        verify_sha256(b"abc", SHA_OF_ABC, "test").unwrap();
    }

    #[test]
    fn bad_example_wrong_checksum_stops_with_a_clear_message() {
        let error = verify_sha256(b"xyz", SHA_OF_ABC, "https://example.test/x.zip").unwrap_err();
        let text = error.to_string();
        assert!(text.contains("Checksum mismatch"), "{text}");
        assert!(text.contains("Nothing was installed"), "{text}");
    }

    #[test]
    fn install_list_is_parsed() {
        let text = "taplo-cli v0.10.0:\n    taplo.exe\ntypos-cli v1.50.3:\n    typos.exe\n";
        let map = parse_install_list(text);
        assert_eq!(map.get("taplo-cli").map(String::as_str), Some("0.10.0"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn binary_is_found_in_zip_and_tar_gz() {
        let mut zip_bytes = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_bytes));
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("dir/tool.exe", options).unwrap();
            writer.write_all(b"zip-binary").unwrap();
            writer.finish().unwrap();
        }
        assert_eq!(
            extract_binary("t.zip", &zip_bytes, "tool.exe").unwrap(),
            b"zip-binary"
        );

        let mut tar_bytes = Vec::new();
        {
            let encoder =
                flate2::write::GzEncoder::new(&mut tar_bytes, flate2::Compression::fast());
            let mut builder = tar::Builder::new(encoder);
            let data = b"tar-binary";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, "pkg/tool", &data[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        assert_eq!(
            extract_binary("t.tar.gz", &tar_bytes, "tool").unwrap(),
            b"tar-binary"
        );
        assert!(extract_binary("t.tar.gz", &tar_bytes, "other").is_err());
    }

    fn fake_tool(sha: &str) -> Tool {
        let entry = format!(
            "[[tool]]\nname = \"tool\"\nkind = \"download\"\nversion = \"1.0.0\"\n\
             windows-x64 = {{ url = \"https://example.test/tool.tar.gz\", sha256 = \"{sha}\" }}\n\
             linux-x64 = {{ url = \"https://example.test/tool.tar.gz\", sha256 = \"{sha}\" }}\n"
        );
        tools::parse(&entry).unwrap().remove(0)
    }

    #[test]
    fn bad_example_download_with_wrong_checksum_installs_nothing() {
        let temp = TempDir::new("setup-bad-checksum");
        let tool = fake_tool(&"0".repeat(64));
        let get = |_: &str, to: &Path| {
            fs::write(to, b"not really an archive")?;
            Ok(())
        };
        let error = install_download_with(temp.path(), &tool, &get).unwrap_err();
        assert!(error.to_string().contains("Checksum mismatch"), "{error}");
        assert!(
            !tools::bin_dir(temp.path())
                .join(tools::exe("tool"))
                .exists()
        );
    }

    #[test]
    fn good_example_download_installs_and_a_running_copy_can_be_replaced() {
        let temp = TempDir::new("setup-good-download");
        let mut tar_bytes = Vec::new();
        {
            let encoder =
                flate2::write::GzEncoder::new(&mut tar_bytes, flate2::Compression::fast());
            let mut builder = tar::Builder::new(encoder);
            let mut header = tar::Header::new_gnu();
            header.set_size(2);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, tools::exe("tool"), &b"v1"[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        let mut digest = String::new();
        for byte in &Sha256::digest(&tar_bytes) {
            let _ = write!(digest, "{byte:02x}");
        }
        let tool = fake_tool(&digest);
        let get = |_: &str, to: &Path| {
            fs::write(to, &tar_bytes)?;
            Ok(())
        };
        let target = tools::bin_dir(temp.path()).join(tools::exe("tool"));
        install_download_with(temp.path(), &tool, &get).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"v1");
        // Second run: nothing to do. An update keeps the old file as `.old` until the next run.
        install_download_with(temp.path(), &tool, &get).unwrap();
        place_binary(&target, b"v2").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"v2");
        assert!(with_suffix(&target, ".old").exists());
        clean_old_binaries(&tools::bin_dir(temp.path()));
        assert!(!with_suffix(&target, ".old").exists());
    }

    #[test]
    fn npm_is_current_only_when_every_package_is_installed_at_the_locked_version() {
        let lock: serde_json::Value = serde_json::from_str(
            r#"{"packages":{"":{},"node_modules/a":{"version":"1.0.0","integrity":"x"}}}"#,
        )
        .unwrap();
        let newer: serde_json::Value = serde_json::from_str(
            r#"{"packages":{"node_modules/a":{"version":"2.0.0","integrity":"x"}}}"#,
        )
        .unwrap();
        let empty = serde_json::json!({"packages":{}});
        assert!(lock_is_installed(&lock, &lock.clone()));
        assert!(!lock_is_installed(&lock, &newer));
        assert!(!lock_is_installed(&lock, &empty));
    }

    #[test]
    fn npm_workspace_folders_do_not_count_as_missing_packages() {
        let lock: serde_json::Value = serde_json::from_str(
            r#"{"packages":{"":{},"web/app":{"version":"0.0.0"},
                "node_modules/a":{"version":"1.0.0","integrity":"x"}}}"#,
        )
        .unwrap();
        let installed: serde_json::Value = serde_json::from_str(
            r#"{"packages":{"node_modules/a":{"version":"1.0.0","integrity":"x"}}}"#,
        )
        .unwrap();
        assert!(lock_is_installed(&lock, &installed));
    }
}
