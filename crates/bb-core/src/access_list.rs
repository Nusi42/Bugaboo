//! Reading and writing the access lists of Windows as text (SEC-16, SEC-18): what `icacls /save`
//! writes (SDDL), the owner of a path as Windows PowerShell writes it (`icacls` does not show
//! owners), the account in what `whoami /user` prints, and the two runs of `icacls` that keep only
//! the entries given. The server (its data folder, its program, its settings and its
//! service) and the release tools (the secret key of a release) read them here once; each runs
//! the system tools itself, because this crate does no I/O.
//!
//! What cannot be read is never taken for private: a list that is missing, an entry of a kind
//! that is not known (a conditional or callback entry) and rights that cannot be read make the
//! answer unknown or open, never closed.

/// The accounts that stand for many people: everybody, all users of the computer, all accounts
/// that signed in, anybody who comes over the network or sits at the computer, guests, every
/// service, every app package, the users of the remote desktop and the power users. Given as the
/// short names SDDL uses and as SIDs. Domain Users and Domain Guests (`S-1-5-21-…-513` and
/// `-514`) are recognised by their ending.
const BROAD: [&str; 23] = [
    "WD",
    "BU",
    "AU",
    "IU",
    "NU",
    "BG",
    "AN",
    "SU",
    "AC",
    "RD",
    "PU",
    "S-1-1-0",
    "S-1-5-32-545",
    "S-1-5-11",
    "S-1-5-4",
    "S-1-5-2",
    "S-1-5-7",
    "S-1-5-32-546",
    "S-1-5-6",
    "S-1-5-80-0",
    "S-1-15-2-1",
    "S-1-5-32-555",
    "S-1-5-32-547",
];

/// The well-known SID of the group of administrators of the computer (fixed, while the name of
/// the group changes with the language).
pub const ADMINISTRATORS_SID: &str = "S-1-5-32-544";

/// The system and the administrators of the computer, as short names and SIDs: the accounts that
/// can take over any file anyway.
pub const SYSTEM_AND_ADMINISTRATORS: [&str; 4] = ["SY", "S-1-5-18", "BA", ADMINISTRATORS_SID];

/// The installer of Windows (`NT SERVICE\TrustedInstaller`), which owns what Windows itself runs.
pub const TRUSTED_INSTALLER_SID: &str =
    "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464";

/// The accounts that may change what Windows itself runs: the system, the administrators of the
/// computer and the installer of Windows.
pub const SYSTEM_ACCOUNTS: [&str; 5] = [
    SYSTEM_AND_ADMINISTRATORS[0],
    SYSTEM_AND_ADMINISTRATORS[1],
    SYSTEM_AND_ADMINISTRATORS[2],
    SYSTEM_AND_ADMINISTRATORS[3],
    TRUSTED_INSTALLER_SID,
];

/// Whether the account or group `who` (a short name of SDDL or a SID) stands for many people.
#[must_use]
pub fn is_broad(who: &str) -> bool {
    BROAD.contains(&who)
        || (who.starts_with("S-1-5-21-") && (who.ends_with("-513") || who.ends_with("-514")))
}

/// One entry of an access list that applies to the object itself, from SDDL text.
#[derive(Debug, PartialEq, Eq)]
pub struct AccessEntry<'a> {
    /// Whether it allows (`A`, `OA`) rather than denies.
    pub allows: bool,
    /// The rights: short names such as `FA` and `GR`, or a number such as `0x1200a9`.
    pub rights: &'a str,
    /// The account or group: a short name of SDDL such as `WD`, or a SID.
    pub who: &'a str,
}

/// The flag of an access list that is there but empty of meaning: a NULL DACL, which lets
/// everybody do everything.
const NULL_LIST: &str = "NO_ACCESS_CONTROL";

/// What a NULL DACL means, as one entry: everybody may do everything.
const EVERYBODY_EVERYTHING: AccessEntry<'static> = AccessEntry {
    allows: true,
    rights: "FA",
    who: "WD",
};

/// The entries of the access list in the SDDL text (the last line `icacls /save` wrote:
/// `D:PAI(A;;FA;;;SY)…`) that apply to the object itself. `None` when the text has no access list
/// at all, or when an entry is of a kind that is not a plain allow or deny (a conditional or
/// callback entry, whose effect cannot be told from the text). A NULL DACL is the one entry
/// "everybody may do everything". An entry that only passes on to what is made inside (inherit
/// only, `IO`) does not apply to the object, and neither does anything of the system list (`S:`)
/// that may follow.
#[must_use]
pub fn access_entries(sddl: &str) -> Option<Vec<AccessEntry<'_>>> {
    let list = sddl
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .and_then(|line| line.strip_prefix("D:"))?;
    let list = list.split("S:").next().unwrap_or_default();
    let flags = list.split('(').next().unwrap_or_default();
    if flags.contains(NULL_LIST) {
        return Some(vec![EVERYBODY_EVERYTHING]);
    }
    let mut entries = Vec::new();
    for entry in list.split('(').skip(1) {
        let entry = entry.split(')').next().unwrap_or_default();
        let parts: Vec<&str> = entry.split(';').collect();
        let (kind, flags, rights, who) =
            (parts.first()?, parts.get(1)?, parts.get(2)?, parts.get(5)?);
        let allows = match *kind {
            "A" | "OA" => true,
            "D" | "OD" => false,
            _ => return None,
        };
        if !flags.contains("IO") {
            entries.push(AccessEntry {
                allows,
                rights,
                who,
            });
        }
    }
    Some(entries)
}

/// The rights of file access that the checks look at.
pub mod rights {
    /// Read the content of a file (`FILE_READ_DATA`), list a folder.
    pub const READ_DATA: u32 = 0x1;
    /// Write a file (`FILE_WRITE_DATA`), add a file to a folder (`FILE_ADD_FILE`).
    pub const WRITE_DATA: u32 = 0x2;
    /// Append to a file (`FILE_APPEND_DATA`), add a folder to a folder (`FILE_ADD_SUBDIRECTORY`).
    pub const APPEND_DATA: u32 = 0x4;
    /// Run a file as a program (`FILE_EXECUTE`).
    pub const EXECUTE: u32 = 0x20;
    /// Delete what is in a folder, whatever its own rights say (`FILE_DELETE_CHILD`).
    pub const DELETE_CHILD: u32 = 0x40;
    /// Delete or rename the object itself (`DELETE`).
    pub const DELETE: u32 = 0x1_0000;
    /// Change the access list (`WRITE_DAC`).
    pub const WRITE_DAC: u32 = 0x4_0000;
    /// Take the object over (`WRITE_OWNER`).
    pub const WRITE_OWNER: u32 = 0x8_0000;
    /// Every right there is: for an entry whose rights cannot be read.
    pub const ALL: u32 = u32::MAX;

    /// What lets an account replace a file or what it holds.
    pub const CHANGE_FILE: u32 = WRITE_DATA | APPEND_DATA | DELETE | WRITE_DAC | WRITE_OWNER;
    /// What lets an account put a file into a folder (a library beside a program) or take one
    /// out of it.
    pub const CHANGE_FOLDER: u32 =
        WRITE_DATA | APPEND_DATA | DELETE_CHILD | DELETE | WRITE_DAC | WRITE_OWNER;
    /// What lets an account replace a folder further up, or what is in it: adding a new name
    /// there replaces nothing.
    pub const REPLACE_BELOW: u32 = DELETE_CHILD | DELETE | WRITE_DAC | WRITE_OWNER;
    /// The same for the root of a drive, which cannot be deleted or renamed itself, so the right
    /// to delete it gives nothing (Windows Sandbox lets every signed-in account modify `C:\`).
    pub const REPLACE_IN_ROOT: u32 = DELETE_CHILD | WRITE_DAC | WRITE_OWNER;
}

/// The generic rights in a number, and the file rights each stands for.
const GENERIC_RIGHTS: [(u32, u32); 5] = [
    (0x1000_0000, FILE_ALL),
    (0x8000_0000, FILE_READ),
    (0x4000_0000, FILE_WRITE),
    (0x2000_0000, FILE_EXECUTE),
    // `MAXIMUM_ALLOWED`: whatever the account can get.
    (0x0200_0000, FILE_ALL),
];

const FILE_ALL: u32 = 0x001F_01FF;
const FILE_READ: u32 = 0x0012_0089;
const FILE_WRITE: u32 = 0x0012_0116;
const FILE_EXECUTE: u32 = 0x0012_00A0;

/// The rights of the short names SDDL writes: for files, the generic rights as Windows maps them
/// for files, and the names of single rights that it writes when no file name fits.
const NAMED_RIGHTS: [(&str, u32); 21] = [
    ("FA", FILE_ALL),
    ("FR", FILE_READ),
    ("FW", FILE_WRITE),
    ("FX", FILE_EXECUTE),
    ("GA", FILE_ALL),
    ("GR", FILE_READ),
    ("GW", FILE_WRITE),
    ("GX", FILE_EXECUTE),
    ("CC", 0x1),
    ("DC", 0x2),
    ("LC", 0x4),
    ("SW", 0x8),
    ("RP", 0x10),
    ("WP", 0x20),
    ("DT", 0x40),
    ("LO", 0x80),
    ("CR", 0x100),
    ("SD", rights::DELETE),
    ("RC", 0x2_0000),
    ("WD", rights::WRITE_DAC),
    ("WO", rights::WRITE_OWNER),
];

/// The rights in the rights part of an SDDL entry, as file rights: a number (`0x1200a9`, its
/// generic rights mapped as for files) or short names of two letters each (`FRFX`). `None` when
/// the text cannot be read, so each caller chooses the safe side.
#[must_use]
pub fn rights_of(text: &str) -> Option<u32> {
    if let Some(number) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        let number = u32::from_str_radix(number, 16).ok()?;
        return Some(
            GENERIC_RIGHTS
                .iter()
                .filter(|(generic, _)| number & generic != 0)
                .fold(number, |all, (_, file)| all | file),
        );
    }
    if text.is_empty() || !text.len().is_multiple_of(2) {
        return None;
    }
    text.as_bytes().chunks(2).try_fold(0, |all, pair| {
        let name = std::str::from_utf8(pair).ok()?;
        NAMED_RIGHTS
            .iter()
            .find_map(|(known, rights)| (*known == name).then_some(all | rights))
    })
}

/// Whether the access list in the SDDL text gives something to one of the accounts that stand
/// for many people; `None` when the text has no access list that can be read, so a caller that
/// cannot look never takes that for "private". A deny entry gives nothing.
#[must_use]
pub fn open_to_many(sddl: &str) -> Option<bool> {
    access_entries(sddl).map(|entries| {
        entries
            .iter()
            .any(|entry| entry.allows && is_broad(entry.who))
    })
}

/// Whether the access list in the SDDL text allows one of `rights` to an account that is not
/// among `trusted` (short names or SIDs): rights that cannot be read count as every right, and a
/// deny entry is not subtracted, so the answer errs on the side of *yes*. `None` when the text
/// has no access list that can be read.
#[must_use]
pub fn allows_others(sddl: &str, trusted: &[&str], rights: u32) -> Option<bool> {
    access_entries(sddl).map(|entries| {
        entries.iter().any(|entry| {
            entry.allows
                && !trusted.contains(&entry.who)
                && rights_of(entry.rights).unwrap_or(rights::ALL) & rights != 0
        })
    })
}

/// The account that stands for the owner of an object in an access list (`OWNER RIGHTS`), as the
/// short name SDDL uses and as its SID. An owner may always change the access list of what it owns
/// and so give itself any right, unless an entry for OWNER RIGHTS applies to the object: then the
/// owner gets only what such entries give.
pub const OWNER_RIGHTS: [&str; 2] = ["OW", "S-1-3-4"];

/// Whether an account that is not among `trusted` may use one of `rights` on an object with this
/// access list and this owner (a SID): an entry allows it to such an account (as
/// [`allows_others`]), or the owner is not trusted and may change the access list (`WRITE_DAC`),
/// which an owner always may unless an entry for [`OWNER_RIGHTS`] applies to the object. An entry
/// for OWNER RIGHTS gives its rights to the owner, so it counts only when the owner is not
/// trusted. `None` when the text has no access list that can be read.
#[must_use]
pub fn changeable_by_others(
    sddl: &str,
    owner: &str,
    trusted: &[&str],
    rights: u32,
) -> Option<bool> {
    let owner_trusted = trusted.contains(&owner);
    let mut also: Vec<&str> = trusted.to_vec();
    if owner_trusted {
        also.extend(OWNER_RIGHTS);
    }
    let by_entry = allows_others(sddl, &also, rights)?;
    let owner_limited = access_entries(sddl)?
        .iter()
        .any(|entry| OWNER_RIGHTS.contains(&entry.who));
    Some(by_entry || (!owner_trusted && !owner_limited && rights & rights::WRITE_DAC != 0))
}

/// The script for Windows PowerShell that writes the owner (a SID) of each of `paths`, one line
/// each and in the same order, or `-` for a path whose owner cannot be read. `icacls` does not
/// show owners. The paths are written into the script as quoted text, never as code.
#[must_use]
pub fn owners_script(paths: &[String]) -> String {
    let quoted: Vec<String> = paths
        .iter()
        .map(|path| format!("'{}'", path.replace('\'', "''")))
        .collect();
    format!(
        "$ErrorActionPreference = 'Stop'; foreach ($path in @({})) {{ try {{ (Get-Item -LiteralPath $path -Force).GetAccessControl('Owner').GetOwner([System.Security.Principal.SecurityIdentifier]).Value }} catch {{ '-' }} }}",
        quoted.join(", ")
    )
}

/// The owners in what [`owners_script`] wrote for `count` paths: the SID of each line, `None` for
/// a line that holds none. Nothing is known when the number of lines is not `count`.
#[must_use]
pub fn owners_in(output: &str, count: usize) -> Vec<Option<String>> {
    let lines: Vec<&str> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() != count {
        return vec![None; count];
    }
    lines
        .into_iter()
        .map(|line| is_sid(line).then(|| line.to_owned()))
        .collect()
}

/// Whether the text is a SID as Windows writes it (`S-1-…`, letters, digits and dashes only).
fn is_sid(text: &str) -> bool {
    text.starts_with("S-1-") && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// The text of the file `icacls /save` wrote, which is UTF-16 with a byte order mark.
#[must_use]
pub fn saved_text(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

/// The SID in the output `"DOMAIN\name","S-1-5-21-…"` of `whoami /user /fo csv /nh`. Names can be
/// translated, renamed or ambiguous; the SID is not.
#[must_use]
pub fn sid_in_whoami(output: &str) -> Option<String> {
    let last = output.trim().rsplit(',').next()?;
    let sid = last.trim().trim_matches('"');
    is_sid(sid).then(|| sid.to_owned())
}

/// The arguments of `whoami.exe` that print the account running it in the form
/// [`sid_in_whoami`] reads.
pub const WHOAMI_ARGUMENTS: [&str; 4] = ["/user", "/fo", "csv", "/nh"];

/// The two runs of `icacls` that keep only `grants` on a path: the first removes every entry of
/// the access list (`/reset`), the second removes inherited rights and grants `grants`.
#[must_use]
pub fn restrict_arguments(grants: &[String]) -> [Vec<String>; 2] {
    let mut grant = vec!["/inheritance:r".to_owned()];
    for entry in grants {
        grant.push("/grant:r".to_owned());
        grant.push(entry.clone());
    }
    [vec!["/reset".to_owned()], grant]
}

/// The entry that gives the account `sid` full control of one file, and nothing to anybody else.
#[must_use]
pub fn owner_only(sid: &str) -> String {
    format!("*{sid}:F")
}

#[cfg(test)]
mod tests;
