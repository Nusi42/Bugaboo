//! The try-out in a fresh WSL distribution with systemd.
//!
//! The distribution is imported for the run from the published Ubuntu image for WSL, which is
//! downloaded once into `.work/try-install/images/` with the list of checksums published beside
//! it and checked against that list at every run. Ubuntu's image switches systemd on in its own
//! `wsl.conf`; the run checks that systemd runs before the first step. The distribution has a name
//! of its own and lives in the folder of the run; it is unregistered at the end, also after a
//! failure. The computer's other distributions are never touched.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, bail};
use bb_core::release::checksum_of;

use super::{Run, write_script};
use crate::output;
use crate::release::assemble::wsl_path;
use crate::setup::{fetch, verify_sha256};
use crate::wsl;

/// The published Ubuntu image for WSL (24.04, the long-term release) and its checksums.
const IMAGE_FOLDER_URL: &str = "https://cloud-images.ubuntu.com/wsl/releases/24.04/current";
const IMAGE_FILE: &str = "ubuntu-noble-wsl-amd64-24.04lts.rootfs.tar.gz";
const CHECKSUMS_FILE: &str = "SHA256SUMS";
/// Where the image is kept between runs, beside the folders of the runs.
const IMAGES: &str = "images";
/// The name of the distribution of the run, and its folder inside the folder of the run.
const DISTRIBUTION: &str = "bugaboo-try-install";
const DISTRIBUTION_FOLDER: &str = "distribution";
/// What `systemctl is-system-running --wait` answers when systemd runs the distribution.
const SYSTEMD_UP: [&str; 2] = ["running", "degraded"];

/// The image, downloaded with its checksums when it is not kept yet, and checked against them.
fn image(images: &Path) -> anyhow::Result<std::path::PathBuf> {
    fs::create_dir_all(images).with_context(|| format!("could not make {}", images.display()))?;
    let image = images.join(IMAGE_FILE);
    let sums = images.join(CHECKSUMS_FILE);
    if !image.is_file() || !sums.is_file() {
        output::info(&format!("try-install: downloading {IMAGE_FILE} ..."));
        fetch(&format!("{IMAGE_FOLDER_URL}/{CHECKSUMS_FILE}"), &sums)?;
        fetch(&format!("{IMAGE_FOLDER_URL}/{IMAGE_FILE}"), &image)?;
    }
    let list =
        fs::read_to_string(&sums).with_context(|| format!("could not read {}", sums.display()))?;
    let expected = checksum_of(&list, IMAGE_FILE)
        .with_context(|| format!("{} names no {IMAGE_FILE}", sums.display()))?;
    let bytes = fs::read(&image).with_context(|| format!("could not read {}", image.display()))?;
    verify_sha256(&bytes, &expected, IMAGE_FILE)?;
    Ok(image)
}

/// `wsl.exe` with `arguments`, its answer as text, and whether it went well.
fn wsl_call(arguments: &[&str]) -> anyhow::Result<(bool, String)> {
    let output = Command::new(wsl::PROGRAM)
        .args(arguments)
        .output()
        .context("could not start wsl.exe: is WSL installed?")?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok((output.status.success(), text))
}

/// The distribution of the run, unregistered when this goes out of scope.
struct Imported;

impl Drop for Imported {
    fn drop(&mut self) {
        if !matches!(wsl_call(&["--unregister", DISTRIBUTION]), Ok((true, _))) {
            output::note(&format!(
                "the distribution {DISTRIBUTION} may still be there: remove it with `wsl --unregister {DISTRIBUTION}`"
            ));
        }
    }
}

/// Imports the distribution, runs the script that `script` writes for the folders as the
/// distribution sees them (the release folder, the folder of the previous program), and removes
/// the distribution.
pub(super) fn run(run: &Run, script: impl Fn(&str, &str) -> String) -> anyhow::Result<()> {
    let images = run
        .work
        .parent()
        .context("the folder of the run has no parent")?
        .join(IMAGES);
    let image = image(&images)?;
    // A distribution left by a run that was cut off goes first.
    let _ = wsl_call(&["--unregister", DISTRIBUTION]);
    let folder = run.work.join(DISTRIBUTION_FOLDER);
    if folder.exists() {
        fs::remove_dir_all(&folder)
            .with_context(|| format!("could not clear {}", folder.display()))?;
    }
    fs::create_dir_all(&folder).with_context(|| format!("could not make {}", folder.display()))?;
    let (imported, said) = wsl_call(&[
        "--import",
        DISTRIBUTION,
        &folder.to_string_lossy(),
        &image.to_string_lossy(),
        "--version",
        "2",
    ])?;
    if !imported {
        bail!("WSL could not import {IMAGE_FILE}: {}", said.trim());
    }
    let distribution = Imported;
    let in_guest = |path: &Path| wsl_path(path, Some(DISTRIBUTION));
    let release = in_guest(&run.release)?;
    let previous = in_guest(&run.previous)?;
    let results = in_guest(&run.results)?;
    write_script(&run.script, &script(&release, &previous))?;
    let (_, state) = wsl_call(&[
        "-d",
        DISTRIBUTION,
        "--exec",
        "systemctl",
        "is-system-running",
        "--wait",
    ])?;
    if !SYSTEMD_UP.contains(&state.trim()) {
        bail!(
            "systemd does not run in the distribution ({}): the image must switch it on in its wsl.conf",
            state.trim()
        );
    }
    output::info("try-install: the distribution runs the steps ...");
    let script_name = run
        .script
        .file_name()
        .context("the script has no name")?
        .to_string_lossy()
        .into_owned();
    let _ = wsl_call(&[
        "-d",
        DISTRIBUTION,
        "-u",
        "root",
        "--cd",
        &results,
        "--exec",
        "bash",
        &script_name,
    ])?;
    drop(distribution);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_04_the_image_is_checked_against_the_published_list() {
        // Ubuntu's list is read as a release's own `SHA256SUMS` is, by one reader.
        let sum = "2a".repeat(32);
        let list = format!(
            "{sum}  {IMAGE_FILE}
{}  other.manifest
",
            "d3".repeat(32)
        );
        let list = list.as_str();
        assert_eq!(checksum_of(list, IMAGE_FILE), Some(sum.clone()));
        assert_eq!(
            checksum_of(
                &format!(
                    "{sum} *{IMAGE_FILE}
"
                ),
                IMAGE_FILE
            ),
            Some(sum)
        );
        assert_eq!(checksum_of(list, "missing.tar.gz"), None);
    }
}
