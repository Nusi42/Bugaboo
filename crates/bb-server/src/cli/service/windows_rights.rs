//! Whether the account of the Windows service can read the program and its settings file
//! (OPS-04, follow-up M1-F57). The service runs as its own virtual account `NT SERVICE\Bugaboo`,
//! not as the person who installs it: a program in that person's user folder is closed to it,
//! and the service would fail at every start. The install looks at the access lists first and
//! refuses with a plain sentence instead.
//!
//! The access list is read as SDDL (`icacls /save`, the same reader as the check of the data
//! folder), and an entry counts when it names the service's own SID or a group that every
//! service account belongs to.

use bb_core::access_list::access_entries;

/// The groups every account of a service belongs to: everybody, the users of the computer, the
/// accounts that signed in, the services, and all services (`NT SERVICE\ALL SERVICES`), as the
/// short names SDDL uses and as SIDs.
const SERVICE_GROUPS: [&str; 9] = [
    "WD",
    "S-1-1-0",
    "BU",
    "S-1-5-32-545",
    "AU",
    "S-1-5-11",
    "SU",
    "S-1-5-6",
    "S-1-5-80-0",
];

/// The right to read the content of a file (`FILE_READ_DATA`).
const READ_DATA: u32 = 0x1;
/// The right to run a file as a program (`FILE_EXECUTE`).
const EXECUTE: u32 = 0x20;

/// The rights of the short names SDDL writes for files: all, read, execute and write, as files
/// and as the generic rights that stand for them.
const NAMED_RIGHTS: [(&str, u32); 8] = [
    ("FA", 0x001F_01FF),
    ("FR", 0x0012_0089),
    ("FX", 0x0012_00A0),
    ("FW", 0x0012_0116),
    ("GA", 0x001F_01FF),
    ("GR", 0x0012_0089),
    ("GX", 0x0012_00A0),
    ("GW", 0x0012_0116),
];

/// What the service does with a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Need {
    /// Reads it (the settings file).
    Read,
    /// Runs it (the program).
    Run,
}

impl Need {
    fn rights(self) -> u32 {
        match self {
            Self::Read => READ_DATA,
            Self::Run => READ_DATA | EXECUTE,
        }
    }
}

/// The rights in the rights part of an SDDL entry: a number (`0x1200a9`) or short names of two
/// letters each (`FRFX`); names that say nothing about reading or running count as none.
fn rights_of(text: &str) -> u32 {
    if let Some(number) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        return u32::from_str_radix(number, 16).unwrap_or(0);
    }
    text.as_bytes()
        .chunks(2)
        .filter_map(|pair| std::str::from_utf8(pair).ok())
        .filter_map(|name| {
            NAMED_RIGHTS
                .iter()
                .find_map(|(known, rights)| (*known == name).then_some(*rights))
        })
        .fold(0, |all, rights| all | rights)
}

/// Whether the access list in `sddl` lets the service whose SID is `service_sid` do `need`: the
/// rights allowed to it and its groups, less every right denied to them. No access list at all
/// counts as no: a check that cannot look must not say all is well.
pub(super) fn service_can(sddl: &str, service_sid: &str, need: Need) -> bool {
    let Some(entries) = access_entries(sddl) else {
        return false;
    };
    let (allowed, denied) = entries
        .iter()
        .filter(|entry| entry.who == service_sid || SERVICE_GROUPS.contains(&entry.who))
        .fold((0, 0), |(allowed, denied), entry| {
            let rights = rights_of(entry.rights);
            if entry.allows {
                (allowed | rights, denied)
            } else {
                (allowed, denied | rights)
            }
        });
    (allowed & !denied) & need.rights() == need.rights()
}

/// The SID in the answer of `sc.exe showsid <name>`, whose words depend on the language of the
/// computer: the one word that is a SID of a service (`S-1-5-80-…`).
pub(super) fn service_sid_in(answer: &str) -> Option<String> {
    answer
        .split_whitespace()
        .find(|word| {
            word.starts_with("S-1-5-80-")
                && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVICE: &str = "S-1-5-80-1-2-3-4-5";
    const PERSON: &str = "S-1-5-21-1-2-3-1001";

    #[test]
    fn ops_04_the_install_refuses_a_program_the_service_account_cannot_read() {
        // A program in a person's user folder: the person, the system and the administrators.
        let profile =
            format!("x\nD:PAI(A;OICI;FA;;;{PERSON})(A;OICI;FA;;;SY)(A;OICI;FA;;;S-1-5-32-544)");
        assert!(!service_can(&profile, SERVICE, Need::Run));
        assert!(!service_can(&profile, SERVICE, Need::Read));
        // A program in Program Files: the users of the computer may read and run it.
        let program_files = "x\nD:AI(A;ID;FA;;;SY)(A;ID;FA;;;S-1-5-32-544)(A;ID;0x1200a9;;;BU)(A;OICIIOID;GXGR;;;BU)";
        assert!(service_can(program_files, SERVICE, Need::Run));
        // The service's own entry counts, also given only to read: then it cannot run it.
        let own = format!("x\nD:PAI(A;;FR;;;{SERVICE})(A;;FA;;;SY)");
        assert!(service_can(&own, SERVICE, Need::Read));
        assert!(!service_can(&own, SERVICE, Need::Run));
        // An entry only for what is made inside, or a deny entry, gives nothing.
        assert!(!service_can(
            "x\nD:PAI(A;OICIIO;GA;;;WD)",
            SERVICE,
            Need::Read
        ));
        assert!(!service_can(
            "x\nD:PAI(D;;FA;;;SU)(A;;FA;;;BU)",
            SERVICE,
            Need::Read
        ));
        // Another service's SID is not this one.
        assert!(!service_can(
            "x\nD:PAI(A;;FA;;;S-1-5-80-9-9-9-9-9)",
            SERVICE,
            Need::Read
        ));
        // Rights that cannot be read, or no access list at all, are a no.
        assert!(!service_can("x\nD:PAI(A;;0xzz;;;BU)", SERVICE, Need::Read));
        assert!(!service_can("", SERVICE, Need::Read));
    }

    #[test]
    fn ops_04_the_sid_of_the_service_is_found_in_any_language() {
        for answer in [
            "\nNAME: Bugaboo\nSERVICE SID: S-1-5-80-1-2-3-4-5\nSTATUS: Active\n",
            "\nNAME: Bugaboo\nDIENST-SID: S-1-5-80-1-2-3-4-5\nSTATUS: Aktiv\n",
        ] {
            assert_eq!(service_sid_in(answer).as_deref(), Some(SERVICE));
        }
        assert_eq!(service_sid_in("[SC] OpenSCManager FAILED 5"), None);
    }
}
