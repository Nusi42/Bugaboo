//! The backup file itself: a tar archive whose first member is the manifest (ADR-0046, decision
//! 1). Writing counts the size and SHA-256 of every member into the manifest; reading checks every
//! name with [`MemberName::check`] before anything is written, puts each member only into the
//! folder it is given, and checks each against the manifest. A backup that fails a check is
//! refused as a whole.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use bb_core::backup::manifest::{BackupProblem, Manifest, Member};
use bb_core::backup::member::{self, EntryKind, MemberName};
use bb_core::security_log::Digest;
use sha2::{Digest as _, Sha256};
use tar::{Archive, Builder, EntryType, Header};

use super::seal;
use crate::error::StoreError;
use crate::keys::KeyRing;

/// The most bytes the seal may have: 64 hex digits and room for a line end.
const SEAL_LIMIT: u64 = 128;

/// The most bytes a manifest may have: far more than one with a million files needs, and a
/// limit on what a crafted file can make the restore read into memory.
const MANIFEST_LIMIT: u64 = 256 * 1024 * 1024;

/// The mode a member is written with: for its owner only, like the data folder.
const MEMBER_MODE: u32 = 0o600;

/// A member to write: its name and the file whose bytes it gets.
pub(crate) struct Source {
    pub(crate) name: MemberName,
    pub(crate) path: PathBuf,
    /// Its size and SHA-256 when the writer of the file counted them already (the copy of
    /// PostgreSQL), so a large file is not read once more only to count them.
    pub(crate) measured: Option<(u64, Digest)>,
}

fn io(error: std::io::Error) -> StoreError {
    StoreError::Backup(error)
}

fn refused(problem: BackupProblem) -> StoreError {
    StoreError::BackupRefused(problem)
}

/// A writer that counts and hashes what goes through it.
pub(crate) struct Counting<W> {
    inner: W,
    hasher: Sha256,
    size: u64,
}

impl<W> Counting<W> {
    pub(crate) fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            size: 0,
        }
    }

    pub(crate) fn finish(self) -> (W, u64, Digest) {
        (
            self.inner,
            self.size,
            Digest::new(self.hasher.finalize().into()),
        )
    }
}

impl<W: Write> Write for Counting<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.hasher.update(&bytes[..written]);
        self.size += u64::try_from(written).unwrap_or(u64::MAX);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// The size and SHA-256 of a file.
pub(crate) fn measure(path: &Path) -> Result<(u64, Digest), StoreError> {
    let mut counting = Counting::new(std::io::sink());
    std::io::copy(&mut File::open(path).map_err(io)?, &mut counting).map_err(io)?;
    let (_, size, digest) = counting.finish();
    Ok((size, digest))
}

/// A new file for the owner only; it fails when the file is there already. On Windows the file
/// takes the access list of its folder, which is the owner's only (the data folder, M1-F65).
pub(crate) fn create_private(path: &Path) -> std::io::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(MEMBER_MODE);
    }
    options.open(path)
}

fn header(size: u64, moment: i64) -> Header {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Regular);
    header.set_size(size);
    header.set_mode(MEMBER_MODE);
    header.set_mtime(u64::try_from(moment).unwrap_or(0));
    header
}

/// Writes the archive `target` (a new file): the manifest first, with the size and checksum of
/// every member counted into it, then its seal under `keys`, then the members. Returns the size and
/// SHA-256 of the archive.
pub(crate) fn write(
    target: &Path,
    manifest: &mut Manifest,
    sources: &[Source],
    keys: &KeyRing,
) -> Result<(u64, Digest), StoreError> {
    manifest.members = sources
        .iter()
        .map(|source| {
            let (size, digest) = match source.measured {
                Some(measured) => measured,
                None => measure(&source.path)?,
            };
            Ok(Member::new(&source.name, size, &digest))
        })
        .collect::<Result<_, StoreError>>()?;
    let json = manifest.to_json().map_err(refused)?;
    let moment = manifest.moment.as_second();
    let file = create_private(target).map_err(io)?;
    let mut builder = Builder::new(Counting::new(BufWriter::new(file)));
    let mut head = header(u64::try_from(json.len()).unwrap_or(u64::MAX), moment);
    builder
        .append_data(&mut head, member::MANIFEST, json.as_slice())
        .map_err(io)?;
    let sealed = seal::seal(keys, &json)?;
    let mut head = header(u64::try_from(sealed.len()).unwrap_or(u64::MAX), moment);
    builder
        .append_data(&mut head, member::SEAL, sealed.as_bytes())
        .map_err(io)?;
    for source in sources {
        let file = File::open(&source.path).map_err(io)?;
        let size = file.metadata().map_err(io)?.len();
        let mut head = header(size, moment);
        builder
            .append_data(&mut head, source.name.as_str(), BufReader::new(file))
            .map_err(io)?;
    }
    let counting = builder.into_inner().map_err(io)?;
    let (writer, size, digest) = counting.finish();
    let file = writer
        .into_inner()
        .map_err(|error| io(error.into_error()))?;
    file.sync_all().map_err(io)?;
    Ok((size, digest))
}

fn entry_kind(kind: EntryType) -> EntryKind {
    match kind {
        EntryType::Regular | EntryType::Continuous => EntryKind::File,
        EntryType::Symlink | EntryType::Link => EntryKind::Link,
        EntryType::Directory => EntryKind::Folder,
        _ => EntryKind::Other,
    }
}

fn entry_name(entry: &tar::Entry<'_, impl Read>) -> Result<String, StoreError> {
    String::from_utf8(entry.path_bytes().into_owned())
        .map_err(|_| refused(BackupProblem::Unreadable))
}

/// The checked name of an entry.
fn checked(entry: &tar::Entry<'_, impl Read>) -> Result<(String, MemberName), StoreError> {
    let name = entry_name(entry)?;
    let member = MemberName::check(&name, entry_kind(entry.header().entry_type()))
        .map_err(|problem| refused(BackupProblem::BadName(name.clone(), problem)))?;
    Ok((name, member))
}

/// The place of a member inside the folder `read` put the members into.
pub(crate) fn extracted(into: &Path, name: &MemberName) -> PathBuf {
    match name.file_parts() {
        Some(parts) => parts
            .into_iter()
            .fold(into.join(member::FILES), |path, part| path.join(part)),
        None => into.join(name.as_str()),
    }
}

/// The bytes of the next entry, which must be `name`, up to `limit` bytes.
fn next_bytes(
    entries: &mut tar::Entries<'_, impl Read>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, StoreError> {
    let mut entry = entries
        .next()
        .ok_or_else(|| refused(BackupProblem::Missing(name.to_owned())))?
        .map_err(|_| refused(BackupProblem::Unreadable))?;
    let (found, _) = checked(&entry)?;
    if found != name {
        return Err(refused(BackupProblem::Missing(name.to_owned())));
    }
    let mut bytes = Vec::new();
    (&mut entry)
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| refused(BackupProblem::Unreadable))?;
    Ok(bytes)
}

/// Reads and checks the archive `archive`, putting its members into the empty folder `into`, and
/// returns its manifest. The seal is checked under `keys` before any member is written: a backup
/// this installation did not seal is refused with nothing written. Nothing outside `into` is
/// written. A backup that fails a check leaves a refusal, and the caller removes `into`.
pub(crate) fn read(archive: &Path, into: &Path, keys: &KeyRing) -> Result<Manifest, StoreError> {
    let file = File::open(archive).map_err(io)?;
    let mut archive = Archive::new(BufReader::new(file));
    let mut entries = archive
        .entries()
        .map_err(|_| refused(BackupProblem::Unreadable))?;
    let bytes = next_bytes(&mut entries, member::MANIFEST, MANIFEST_LIMIT)?;
    let manifest = Manifest::read(&bytes).map_err(refused)?;
    let sealed = next_bytes(&mut entries, member::SEAL, SEAL_LIMIT)
        .map_err(|_| StoreError::BackupNotSealed)?;
    if !seal::sealed(keys, &bytes, &sealed) {
        return Err(StoreError::BackupNotSealed);
    }
    let mut found: Vec<MemberName> = Vec::new();
    for entry in entries {
        let mut entry = entry.map_err(|_| refused(BackupProblem::Unreadable))?;
        let (name, member) = checked(&entry)?;
        if found.contains(&member) || manifest.member(member.as_str()).is_none() {
            return Err(refused(BackupProblem::Extra(name)));
        }
        let place = extracted(into, &member);
        if let Some(parent) = place.parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        let mut counting = Counting::new(BufWriter::new(create_private(&place).map_err(io)?));
        std::io::copy(&mut entry, &mut counting)
            .map_err(|_| refused(BackupProblem::Damaged(name.clone())))?;
        let (writer, size, digest) = counting.finish();
        writer
            .into_inner()
            .map_err(|error| io(error.into_error()))?
            .sync_all()
            .map_err(io)?;
        manifest
            .check_member(&member, size, &digest)
            .map_err(refused)?;
        found.push(member);
    }
    manifest.check_complete(&found).map_err(refused)?;
    Ok(manifest)
}

#[cfg(test)]
#[path = "../tests/backup_archive.rs"]
mod tests;
