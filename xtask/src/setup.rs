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

use crate::output;
use crate::repo::Repo;
use crate::tools::{self, Tool};

/// Installs or updates every tool in `tools/dev/tools.toml`, then the npm packages.
pub(crate) fn run(repo: &Repo) -> anyhow::Result<()> {
    let list = tools::load(repo)?;
    let root = repo.root();
    fs::create_dir_all(tools::bin_dir(root))?;
    let installed = installed_cargo_tools(root)?;
    for tool in &list {
        match tool.kind.as_str() {
            "cargo" => install_cargo(root, tool, &installed)?,
            "download" => install_download(root, tool)?,
            other => bail!(
                "{}: unknown kind `{other}` in {}; use `cargo` or `download`.",
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
    if installed.get(crate_name) == Some(&tool.version) {
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
            "cargo install of {crate_name} {} failed (see the messages above).",
            tool.version
        );
    }
    Ok(())
}

fn install_download(root: &Path, tool: &Tool) -> anyhow::Result<()> {
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
    fetch(&download.url, &archive)?;
    let bytes = fs::read(&archive)?;
    if let Err(error) = verify_sha256(&bytes, &download.sha256, &download.url) {
        // A bad file must never be reused on the next run.
        let _ = fs::remove_file(&archive);
        return Err(error);
    }
    let contents = extract_binary(file_name, &bytes, &binary)?;
    fs::create_dir_all(tools::bin_dir(root))?;
    fs::write(&target, contents)?;
    make_executable(&target)?;
    if let Some(folder) = marker.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(&marker, &tool.version)?;
    Ok(())
}

/// Downloads `url` to `destination` with curl (present on every supported system).
fn fetch(url: &str, destination: &Path) -> anyhow::Result<()> {
    let status = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
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

#[cfg(unix)]
fn make_executable(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
#[allow(
    clippy::unnecessary_wraps,
    reason = "same signature as the Unix version (setup.rs)"
)]
fn make_executable(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

/// `npm ci` when the repository has a root `package.json`.
fn install_npm(root: &Path) -> anyhow::Result<()> {
    if !root.join("package.json").is_file() {
        return Ok(());
    }
    output::info("install  npm packages (npm ci)");
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(npm)
        .arg("ci")
        .current_dir(root)
        .status()
        .context("could not run npm; is Node.js installed?")?;
    if !status.success() {
        bail!("npm ci failed (see the messages above).");
    }
    Ok(())
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

    #[test]
    fn bad_example_download_with_wrong_checksum_installs_nothing() {
        let temp = TempDir::new("setup-bad-checksum");
        let archive = temp.write("source/tool.tar.gz", b"not really an archive");
        let url = format!(
            "file:///{}",
            archive
                .to_string_lossy()
                .replace('\\', "/")
                .trim_start_matches('/')
        );
        let zeros = "0".repeat(64);
        let entry = format!(
            "[[tool]]\nname = \"tool\"\nkind = \"download\"\nversion = \"1.0.0\"\n\
             windows-x64 = {{ url = \"{url}\", sha256 = \"{zeros}\" }}\n\
             linux-x64 = {{ url = \"{url}\", sha256 = \"{zeros}\" }}\n"
        );
        let list = tools::parse(&entry).unwrap();
        let error = install_download(temp.path(), &list[0]).unwrap_err();
        assert!(error.to_string().contains("Checksum mismatch"), "{error}");
        assert!(!tools::bin_dir(temp.path()).join("tool").exists());
        assert!(!tools::bin_dir(temp.path()).join("tool.exe").exists());
    }
}
