//! `bugaboo update verify <file>` (DIST-07, SEC-18; ADR-0047 decisions 2 and 8): the program that
//! is installed checks a downloaded file of a release before anybody installs it.
//!
//! It trusts only the keys it carries itself (`bb_core::release::RELEASE_PUBLIC_KEYS`), so a
//! download cannot bring its own key. It checks the signature of the list `SHA256SUMS` beside the
//! file (signed as that list of a release; the one signature of a release) and the file's checksum
//! in that list, says which release and which file of it the file is, and fails when that release
//! is older than the installed one, unless `--allow-older` says that is meant. It only reads:
//! nothing is run, moved or replaced.

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
    #[arg(long = ALLOW_OLDER, help = text(message::CLI_HELP_ALLOW_OLDER))]
    allow_older: bool,
}

/// The option that lets `update verify` pass a release older than the installed one.
const ALLOW_OLDER: &str = "allow-older";

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
    /// The list of checksums beside the file is signed as another file (the file and the version
    /// the signature names).
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
    verify_with(args, release::RELEASE_PUBLIC_KEYS, release::VERSION).map(|_| ())
}

/// The same with the keys and the installed version given (the tests). A genuine release that is
/// older than the installed one fails too, unless the person said they mean it: going back can
/// undo fixes of security, and a script that only looks at the exit code must not take it for
/// an update.
fn verify_with(args: &VerifyArgs, keys: &[ReleaseKey], installed: &str) -> anyhow::Result<Checked> {
    let checked = check(&args.file, keys, installed)
        .map_err(|refused| anyhow!(refused.message().render()))?;
    if checked.older && !args.allow_older {
        return Err(anyhow!(
            message::cli_update_older_refused(message::CliUpdateOlderRefused {
                flag: &format!("--{ALLOW_OLDER}"),
                installed: &installed,
                version: &checked.signed.version,
            })
            .render()
        ));
    }
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
                installed: &installed,
                version: &checked.signed.version,
            })
            .render(),
        );
    }
    Ok(checked)
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

/// Checks the file at `path` against the list of checksums beside it, whose signature is checked
/// against `keys`, for a program of the version `installed`. Only the list is signed: it names
/// every file of the release with its checksum.
fn check(path: &Path, keys: &[ReleaseKey], installed: &str) -> Result<Checked, Refused> {
    let bytes = read(path)?;
    let folder = path.parent().unwrap_or_else(|| Path::new(""));
    let (sums, comment) = read_signed(&folder.join(CHECKSUMS_FILE), keys)?;
    let list = TrustedComment::read(&comment)
        .ok_or_else(|| Refused::NoRelease(CHECKSUMS_FILE.to_owned()))?;
    // The signature must be the one of the list, so a signed file of another kind cannot stand in
    // for it.
    if list.file != CHECKSUMS_FILE {
        return Err(Refused::OtherList(name_of(path), list.version));
    }
    // The list names the file as the release does, so a download saved under another name is
    // still found by its checksum.
    let sums = String::from_utf8_lossy(&sums);
    let sum = bb_core::hex::encode(&Sha256::digest(&bytes));
    let file = release::checksum_entries(&sums)
        .into_iter()
        .find_map(|(listed, file)| (listed == sum).then_some(file))
        .ok_or_else(|| {
            let name = name_of(path);
            if release::checksum_of(&sums, &name).is_some() {
                Refused::OtherChecksum(name)
            } else {
                Refused::NotListed(name)
            }
        })?;
    let older = release::is_older(&list.version, installed) == Some(true);
    Ok(Checked {
        signed: TrustedComment {
            file,
            version: list.version,
        },
        older,
    })
}

#[cfg(test)]
mod tests;
