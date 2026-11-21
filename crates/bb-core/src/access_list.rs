//! Reading and writing the access lists of Windows as text (SEC-16, SEC-18): what `icacls /save`
//! writes (SDDL), the account in what `whoami /user` prints, and the two runs of `icacls` that
//! keep only the entries given. The server (its data folder and its service) and the release
//! tools (the secret key of a release) read them here once; each runs the system tools itself,
//! because this crate does no I/O.

/// The accounts that stand for many people: everybody, all users of the computer, all accounts
/// that signed in, anybody who comes over the network or sits at the computer, guests. Given as
/// the short names SDDL uses and as SIDs. Domain Users and Domain Guests (`S-1-5-21-…-513` and
/// `-514`) are recognised by their ending.
const BROAD: [&str; 14] = [
    "WD",
    "BU",
    "AU",
    "IU",
    "NU",
    "BG",
    "AN",
    "S-1-1-0",
    "S-1-5-32-545",
    "S-1-5-11",
    "S-1-5-4",
    "S-1-5-2",
    "S-1-5-7",
    "S-1-5-32-546",
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

/// The entries of the access list in the SDDL text (the last line `icacls /save` wrote:
/// `D:PAI(A;;FA;;;SY)…`) that apply to the object itself, `None` when the text has no access list
/// at all. An entry that only passes on to what is made inside (inherit only, `IO`) does not
/// apply to the object, and neither does anything of the system list (`S:`) that may follow.
#[must_use]
pub fn access_entries(sddl: &str) -> Option<Vec<AccessEntry<'_>>> {
    let list = sddl
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .and_then(|line| line.strip_prefix("D:"))?;
    let list = list.split("S:").next().unwrap_or_default();
    Some(
        list.split('(')
            .skip(1)
            .filter_map(|entry| {
                let entry = entry.split(')').next().unwrap_or_default();
                let parts: Vec<&str> = entry.split(';').collect();
                let (kind, flags, rights, who) =
                    (parts.first()?, parts.get(1)?, parts.get(2)?, parts.get(5)?);
                let allows = match *kind {
                    "A" | "OA" => true,
                    "D" | "OD" => false,
                    _ => return None,
                };
                (!flags.contains("IO")).then_some(AccessEntry {
                    allows,
                    rights,
                    who,
                })
            })
            .collect(),
    )
}

/// Whether the access list in the SDDL text gives something to one of the accounts that stand
/// for many people; `None` when the text has no access list at all, so a caller that cannot look
/// never takes that for "private". A deny entry gives nothing.
#[must_use]
pub fn open_to_many(sddl: &str) -> Option<bool> {
    access_entries(sddl).map(|entries| {
        entries
            .iter()
            .any(|entry| entry.allows && is_broad(entry.who))
    })
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
    (sid.starts_with("S-1-") && sid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .then(|| sid.to_owned())
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
mod tests {
    use super::*;

    #[test]
    fn sec_16_an_access_list_is_open_when_it_gives_something_to_many() {
        assert_eq!(
            open_to_many("x\nD:PAI(A;;FA;;;S-1-5-21-1-2-3-1001)(A;;FA;;;SY)"),
            Some(false)
        );
        for who in ["WD", "BU", "AU", "S-1-1-0", "S-1-5-21-111-222-333-513"] {
            let open = format!("x\nD:PAI(A;;FA;;;SY)(A;;0x1200a9;;;{who})");
            assert_eq!(open_to_many(&open), Some(true), "{who}");
        }
        // Inherit only, a deny entry and the system list give nothing to the object.
        assert_eq!(open_to_many("x\nD:PAI(A;OICIIO;GR;;;WD)"), Some(false));
        assert_eq!(open_to_many("x\nD:PAI(D;;FA;;;WD)"), Some(false));
        assert_eq!(
            open_to_many("x\nD:PAI(A;;FA;;;SY)S:AI(A;;FA;;;WD)"),
            Some(false)
        );
        // A path on drive D is not the list, and no list is no answer.
        assert_eq!(open_to_many("D:\\data\nD:PAI(A;;FA;;;SY)"), Some(false));
        assert_eq!(open_to_many(""), None);
    }

    #[test]
    fn sec_16_the_saved_list_and_the_account_are_read() {
        let bytes: Vec<u8> = "\u{feff}x\r\nD:PAI(A;;FA;;;SY)"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(saved_text(&bytes).ends_with("D:PAI(A;;FA;;;SY)"));
        assert_eq!(
            sid_in_whoami("\"EXAMPLE-PC\\ana\",\"S-1-5-21-1-2-3-1001\"\r\n").as_deref(),
            Some("S-1-5-21-1-2-3-1001")
        );
        assert_eq!(sid_in_whoami("nonsense"), None);
        assert_eq!(sid_in_whoami("\"a\",\"S-1-5-21; calc\""), None);
    }

    #[test]
    fn sec_18_restricting_drops_every_entry_and_grants_only_the_given() {
        assert_eq!(
            restrict_arguments(&[owner_only("S-1-5-21-1")]),
            [
                vec!["/reset".to_owned()],
                vec![
                    "/inheritance:r".to_owned(),
                    "/grant:r".to_owned(),
                    "*S-1-5-21-1:F".to_owned()
                ]
            ]
        );
    }
}
