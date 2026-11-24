//! `cargo xtask release sign` and `cargo xtask release keygen` (SEC-18, ADR-0047 decision 2, as
//! amended): the list `SHA256SUMS` of a release, which names every other file of it with its
//! checksum, is signed with the release key in the minisign format, and the two keys are made,
//! with the `minisign` crate and no signature code of our own.
//!
//! A secret key never enters the repository or a work folder: both commands refuse a path inside
//! this repository or inside any Git working tree (every work folder and the main checkout are
//! one), before anything is read or written. The keys have no passphrase (ADR-0047, decision 2,
//! as amended), so the access list of the file is what keeps them: `keygen` makes the file
//! readable by its owner only and asks nothing, and `sign` refuses a key file other accounts may
//! read and opens any other plain key without a question. An older
//! key with a passphrase still opens; only then does `minisign` itself ask for the passphrase on
//! the console, and this code never sees, keeps or prints it.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use minisign::{KeyPair, PublicKey, SecretKey, SecretKeyBox};

use bb_core::release::{CHECKSUMS_FILE, RELEASE_PUBLIC_KEYS};

use super::folder::{self, Build};
use super::parts_list::base64_decode;

/// The variable that may name the secret key instead of `--key`.
pub(crate) const KEY_VARIABLE: &str = "BUGABOO_RELEASE_KEY";

/// The comment line of a secret key that `keygen` writes.
const SECRET_COMMENT: &str = "secret key of a Bugaboo release";

/// The ending of the public key that `keygen` writes beside the secret key.
const PUBLIC_END: &str = ".pub";

/// Refuses `key` when it lies inside the repository at `root` or inside any Git working tree. The
/// path is made absolute and its links are followed first (as far as it exists), so `..` and a
/// link do not hide where it is.
pub(crate) fn refuse_inside_a_folder_of_work(key: &Path, root: &Path) -> anyhow::Result<()> {
    let real = real_path(key)?;
    let real_root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let inside_git = real
        .ancestors()
        .skip(1)
        .any(|folder| folder.join(".git").exists());
    if real.starts_with(&real_root) || inside_git {
        bail!(
            "{} lies inside the repository or a work folder (a Git working tree). A secret key of \
             a release is kept outside the repository and every work folder (ADR-0047, decision \
             2): move it to a place outside them and name that.",
            key.display()
        );
    }
    Ok(())
}

/// The real path of `path`: of the path itself when it exists, else of the nearest folder above
/// it that does, with the rest joined on.
fn real_path(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute = std::path::absolute(path)
        .with_context(|| format!("could not make {} absolute", path.display()))?;
    let mut rest = Vec::new();
    let mut existing = absolute.as_path();
    loop {
        if let Ok(real) = fs::canonicalize(existing) {
            return Ok(rest.iter().rev().fold(real, |path, part| path.join(part)));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                existing = parent;
            }
            _ => return Ok(absolute),
        }
    }
}

/// The secret key named by `--key`, else by [`KEY_VARIABLE`].
pub(crate) fn key_path(given: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    given
        .or_else(|| std::env::var_os(KEY_VARIABLE).map(PathBuf::from))
        .with_context(|| {
            format!("name the secret key of the release with --key <path> or {KEY_VARIABLE}")
        })
}

/// Opens the secret key at `path`, after refusing a place in a folder of work and a file that
/// other accounts may read. A key without a passphrase opens without a question; for an older
/// key with one, `minisign` asks for it on the console.
pub(crate) fn open_key(path: &Path, root: &Path) -> anyhow::Result<SecretKey> {
    refuse_inside_a_folder_of_work(path, root)?;
    private::refuse_open_to_others(path)?;
    let text = fs::read_to_string(path)
        .with_context(|| format!("could not read the secret key {}", path.display()))?;
    key_of(&text, None).with_context(|| format!("could not open the secret key {}", path.display()))
}

/// The secret key in `text`. Only a key whose KDF field says that it is locked with a passphrase
/// is opened with `passphrase`, or, when that is `None`, with the passphrase `minisign` asks for
/// on the console; any other key is read as a plain key, so a damaged one is an error and never a
/// question.
pub(crate) fn key_of(text: &str, passphrase: Option<String>) -> anyhow::Result<SecretKey> {
    let key_box = SecretKeyBox::from_string(text)
        .map_err(|error| anyhow::anyhow!("not a minisign secret key: {error}"))?;
    if has_passphrase(text) {
        SecretKey::from_box(key_box, passphrase)
    } else {
        SecretKey::from_unencrypted_box(key_box)
    }
    .map_err(|error| anyhow::anyhow!("{error}"))
}

/// Whether the key in `text` (a comment line and the key in Base64) is locked with a passphrase,
/// as its KDF field says.
fn has_passphrase(text: &str) -> bool {
    text.lines()
        .nth(1)
        .and_then(|line| base64_decode(line.trim()))
        .and_then(|bytes| SecretKey::from_bytes(&bytes).ok())
        .is_some_and(|key| key.is_encrypted())
}

/// Whether `public` is one of the keys the program carries.
pub(crate) fn is_ours(public: &str) -> bool {
    RELEASE_PUBLIC_KEYS
        .iter()
        .any(|known| known.public == public)
}

/// Why `release sign` must not sign the build `build` of `version` at all, if it must not: while the
/// program trusts only test keys, anything is signed (and marked a test build); once it carries the
/// real keys, a release is signed only from its tag. Asked before the key is opened, so nobody
/// types the passphrase for a build that is refused anyway.
pub(crate) fn tag_refusal(build: Build, version: &str) -> Option<String> {
    (!build.test_keys && !build.tagged).then(|| {
        format!(
            "the tag v{version} is not on this commit: a release signed with a real key is built \
             from its tag. Check out the tag and run `cargo xtask release` again."
        )
    })
}

/// Why `release sign` must not sign the build `build` with the key whose public key is `public`,
/// if it must not: once the program carries the real keys, only one of those keys signs, or
/// installed programs would refuse the files.
pub(crate) fn key_refusal(build: Build, public: &str) -> Option<String> {
    (!build.test_keys && !is_ours(public)).then(|| {
        format!(
            "the key {public} is not one of RELEASE_PUBLIC_KEYS: installed programs would refuse \
             these files. Sign with the release key or the backup key."
        )
    })
}

/// Signs the release named `release` in `folder` with `key`: writes `SHA256SUMS` for its payload
/// and signs that list, the one signature of a release (`SHA256SUMS.minisig`), whose trusted
/// comment names the list and the release.
pub(crate) fn sign_folder(folder: &Path, release: &str, key: &SecretKey) -> anyhow::Result<()> {
    folder::write_checksums(folder, release)?;
    let public = PublicKey::from_secret_key(key)
        .map_err(|error| anyhow::anyhow!("could not read the public key: {error}"))?;
    let name = CHECKSUMS_FILE;
    let path = folder.join(name);
    let bytes = fs::read(&path).with_context(|| format!("could not read {}", path.display()))?;
    let signature = minisign::sign(
        Some(&public),
        key,
        bytes.as_slice(),
        Some(&folder::trusted_comment(name, release)),
        None,
    )
    .map_err(|error| anyhow::anyhow!("could not sign {name}: {error}"))?;
    super::write_file(
        folder,
        &bb_core::release::signature_name(name),
        &signature.into_string(),
    )
}

/// The public key of `key`, in the form `RELEASE_PUBLIC_KEYS` holds.
pub(crate) fn public_of(key: &SecretKey) -> anyhow::Result<String> {
    Ok(PublicKey::from_secret_key(key)
        .map_err(|error| anyhow::anyhow!("could not read the public key: {error}"))?
        .to_base64())
}

/// `cargo xtask release keygen --out <path>`: makes a key pair without a passphrase and asks
/// nothing: the secret key at `out` and the public key beside it at `<out>.pub`, and gives back the
/// public key. Refuses a place inside a folder of work and a key that is there already.
pub(crate) fn keygen(out: &Path, root: &Path) -> anyhow::Result<String> {
    refuse_inside_a_folder_of_work(out, root)?;
    let mut public_path = out.as_os_str().to_owned();
    public_path.push(PUBLIC_END);
    let public_path = PathBuf::from(public_path);
    for path in [out, public_path.as_path()] {
        if path.exists() {
            bail!(
                "{} is there already; a key is never written over",
                path.display()
            );
        }
    }
    // The two files in the text form of minisign (a comment line and the key), the form `sign`
    // and the `minisign` tool read; the library's own writer of plain keys writes raw bytes.
    let pair = KeyPair::generate_unencrypted_keypair()
        .map_err(|error| anyhow::anyhow!("could not make the key pair: {error}"))?;
    let secret_text = pair
        .sk
        .to_box(Some(SECRET_COMMENT))
        .map_err(|error| anyhow::anyhow!("could not write the secret key: {error}"))?
        .into_string();
    let public_text = pair
        .pk
        .to_box()
        .map_err(|error| anyhow::anyhow!("could not write the public key: {error}"))?
        .into_string();
    // The secret key file is made private before anything goes into it.
    let mut secret = private::create(out)?;
    std::io::Write::write_all(&mut secret, secret_text.as_bytes())
        .with_context(|| format!("could not write {}", out.display()))?;
    let mut public = fs::File::create_new(&public_path)
        .with_context(|| format!("could not create {}", public_path.display()))?;
    std::io::Write::write_all(&mut public, public_text.as_bytes())
        .with_context(|| format!("could not write {}", public_path.display()))?;
    Ok(pair.pk.to_base64())
}

mod private;
#[cfg(test)]
mod tests;
