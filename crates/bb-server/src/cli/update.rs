//! `bugaboo update verify <file>` (DIST-07, SEC-18; ADR-0047 decisions 2 and 8): the program that
//! is installed checks a downloaded file of a release before anybody installs it.
//!
//! It trusts only the keys it carries itself (`bb_core::release::RELEASE_PUBLIC_KEYS`), so a
//! download cannot bring its own key. It checks the file's signature, the signature of the list
//! `SHA256SUMS` beside it (signed as that list of the same release) and the file's checksum in
//! that list, says which release the file is, and warns when that release is older than the
//! installed one. It only reads: nothing is run, moved or replaced.

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use bb_core::message::{self, Message};
use bb_core::release::{self, CHECKSUMS_FILE, ReleaseKey, SignatureProblem, TrustedComment};
use clap::{Args, Subcommand};
use sha2::{Digest as _, Sha256};

use super::text;
use crate::output;

#[derive(Subcommand, Clone)]
pub(crate) enum UpdateCommand {
    #[command(about = text(message::CLI_ABOUT_UPDATE_VERIFY))]
    Verify(VerifyArgs),
}

/// What `bugaboo update verify` is told.
#[derive(Args, Clone)]
pub(crate) struct VerifyArgs {
    #[arg(help = text(message::CLI_HELP_UPDATE_FILE))]
    file: PathBuf,
}

/// A file that holds: the release it belongs to, and whether that is older than this program.
#[derive(Debug, PartialEq, Eq)]
struct Checked {
    signed: TrustedComment,
    older: bool,
}

/// Why a file is refused.
#[derive(Debug, PartialEq, Eq)]
enum Refused {
    /// A file that is needed cannot be read.
    Missing(String),
    /// A signature does not hold.
    Signature(String, SignatureProblem),
    /// The signature names no release of ours.
    NoRelease(String),
    /// The list of checksums beside the file is signed as another file or for another release.
    OtherList(String, String),
    /// The list of checksums does not name the file.
    NotListed(String),
    /// The checksum of the file is not the one in the list.
    OtherChecksum(String),
}

impl Refused {
    fn message(&self) -> Message {
        match self {
            Self::Missing(file) => message::cli_update_missing(file),
            Self::Signature(file, SignatureProblem::Unreadable) => {
                message::cli_update_unreadable_signature(file)
            }
            Self::Signature(file, SignatureProblem::UnknownKey) => {
                message::cli_update_unknown_key(file)
            }
            Self::Signature(file, SignatureProblem::Changed) => message::cli_update_changed(file),
            Self::NoRelease(file) => message::cli_update_no_release(file),
            Self::OtherList(file, version) => {
                message::cli_update_other_list(message::CliUpdateOtherList {
                    checksums: &CHECKSUMS_FILE,
                    file: &file,
                    version: &version,
                })
            }
            Self::NotListed(file) => message::cli_update_not_listed(message::CliUpdateNotListed {
                checksums: &CHECKSUMS_FILE,
                file: &file,
            }),
            Self::OtherChecksum(file) => {
                message::cli_update_other_checksum(message::CliUpdateOtherChecksum {
                    checksums: &CHECKSUMS_FILE,
                    file: &file,
                })
            }
        }
    }
}

/// `bugaboo update verify <file>`: checks the file with the keys of this program.
pub(crate) fn verify(args: &VerifyArgs) -> anyhow::Result<()> {
    match check(&args.file, release::RELEASE_PUBLIC_KEYS, release::VERSION) {
        Ok(checked) => {
            output::line(
                &message::cli_update_signed(message::CliUpdateSigned {
                    file: &checked.signed.file,
                    version: &checked.signed.version,
                })
                .render(),
            );
            if checked.older {
                output::warning(
                    &message::cli_update_older(message::CliUpdateOlder {
                        installed: &release::VERSION,
                        version: &checked.signed.version,
                    })
                    .render(),
                );
            }
            Ok(())
        }
        Err(refused) => Err(anyhow!(refused.message().render())),
    }
}

/// The name of a file as text, for the messages.
fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The bytes of `path`, or why they cannot be had.
fn read(path: &Path) -> Result<Vec<u8>, Refused> {
    std::fs::read(path).map_err(|_| Refused::Missing(path.display().to_string()))
}

/// The bytes of `path` and the trusted comment of its signature beside it
/// ([`release::signature_name`]).
fn read_signed(path: &Path, keys: &[ReleaseKey]) -> Result<(Vec<u8>, String), Refused> {
    let bytes = read(path)?;
    let signature_path = path.with_file_name(release::signature_name(&name_of(path)));
    let signature = String::from_utf8(read(&signature_path)?)
        .map_err(|_| Refused::Signature(name_of(&signature_path), SignatureProblem::Unreadable))?;
    let comment = release::verify(&bytes, &signature, keys)
        .map_err(|problem| Refused::Signature(name_of(path), problem))?;
    Ok((bytes, comment))
}

/// Checks the file at `path` and the list of checksums beside it against `keys`, for a program of
/// the version `installed`.
fn check(path: &Path, keys: &[ReleaseKey], installed: &str) -> Result<Checked, Refused> {
    let (bytes, comment) = read_signed(path, keys)?;
    let signed = TrustedComment::read(&comment).ok_or_else(|| Refused::NoRelease(name_of(path)))?;
    let folder = path.parent().unwrap_or_else(|| Path::new(""));
    let (sums, sums_comment) = read_signed(&folder.join(CHECKSUMS_FILE), keys)?;
    // The list must be signed as the list of the same release, so the list of another release
    // cannot vouch for this file.
    let list = TrustedComment {
        file: CHECKSUMS_FILE.to_owned(),
        version: signed.version.clone(),
    };
    if TrustedComment::read(&sums_comment).as_ref() != Some(&list) {
        return Err(Refused::OtherList(signed.file, signed.version));
    }
    // The list names the file as the release does, so a download saved under another name is
    // still found.
    let listed = release::checksum_of(&String::from_utf8_lossy(&sums), &signed.file)
        .ok_or_else(|| Refused::NotListed(signed.file.clone()))?;
    if bb_core::hex::encode(&Sha256::digest(&bytes)) != listed {
        return Err(Refused::OtherChecksum(signed.file));
    }
    let older = release::is_older(&signed.version, installed) == Some(true);
    Ok(Checked { signed, older })
}

#[cfg(test)]
mod tests;
