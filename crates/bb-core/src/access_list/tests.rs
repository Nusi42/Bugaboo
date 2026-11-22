use super::*;

#[test]
fn sec_16_an_access_list_is_open_when_it_gives_something_to_many() {
    assert_eq!(
        open_to_many("x\nD:PAI(A;;FA;;;S-1-5-21-1-2-3-1001)(A;;FA;;;SY)"),
        Some(false)
    );
    for who in [
        "WD",
        "BU",
        "AU",
        "S-1-1-0",
        "S-1-5-21-111-222-333-513",
        // Every service, every app package, the remote desktop and the power users.
        "SU",
        "S-1-5-6",
        "S-1-5-80-0",
        "AC",
        "S-1-15-2-1",
        "RD",
        "S-1-5-32-555",
        "PU",
        "S-1-5-32-547",
    ] {
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
fn sec_16_a_null_list_is_open_and_an_unknown_kind_of_entry_is_never_private() {
    // A NULL DACL lets everybody do everything.
    assert_eq!(open_to_many("x\nD:NO_ACCESS_CONTROL"), Some(true));
    assert_eq!(open_to_many("x\nD:PNO_ACCESS_CONTROL"), Some(true));
    assert_eq!(
        allows_others(
            "x\nD:NO_ACCESS_CONTROL",
            &SYSTEM_ACCOUNTS,
            rights::CHANGE_FILE
        ),
        Some(true)
    );
    // An empty list lets nobody in.
    assert_eq!(open_to_many("x\nD:P"), Some(false));
    // A conditional or callback entry cannot be judged from the text.
    for kind in ["XA", "XD", "ZA", "XU", "ML"] {
        let list = format!("x\nD:PAI(A;;FA;;;SY)({kind};;FA;;;WD;(Member_of {{SID(BA)}}))");
        assert_eq!(open_to_many(&list), None, "{kind}");
        assert_eq!(
            allows_others(&list, &SYSTEM_ACCOUNTS, rights::CHANGE_FILE),
            None,
            "{kind}"
        );
    }
}

#[test]
fn sec_16_the_rights_of_an_entry_are_read_as_file_rights() {
    assert_eq!(rights_of("FA"), Some(0x001F_01FF));
    assert_eq!(rights_of("FRFX"), Some(0x0012_00A9));
    assert_eq!(rights_of("0x1200a9"), Some(0x0012_00A9));
    // Generic rights in a number are mapped as for files.
    assert_eq!(rights_of("0x10000000"), Some(0x1000_0000 | 0x001F_01FF));
    assert_eq!(
        rights_of("0x40000000").map(|r| r & rights::WRITE_DATA),
        Some(rights::WRITE_DATA)
    );
    // The single rights SDDL writes when no file name fits.
    assert_eq!(
        rights_of("WDWO"),
        Some(rights::WRITE_DAC | rights::WRITE_OWNER)
    );
    assert_eq!(rights_of("SD"), Some(rights::DELETE));
    assert_eq!(rights_of("DC"), Some(rights::WRITE_DATA));
    // What cannot be read is no answer.
    for text in ["", "0xzz", "F", "KAKR", "FAX"] {
        assert_eq!(rights_of(text), None, "{text}");
    }
}

#[test]
fn ops_04_others_who_can_change_a_program_are_found() {
    const PERSON: &str = "S-1-5-21-1-2-3-1001";
    // Program Files: the system, the administrators and the installer of Windows change it; the
    // users read and run it.
    let program_files = "x\nD:AI(A;ID;FA;;;S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464)(A;ID;FA;;;SY)(A;ID;FA;;;BA)(A;ID;0x1200a9;;;BU)(A;ID;0x1200a9;;;AC)(A;OICIIOID;GA;;;CO)";
    for change in [
        rights::CHANGE_FILE,
        rights::CHANGE_FOLDER,
        rights::REPLACE_BELOW,
    ] {
        assert_eq!(
            allows_others(program_files, &SYSTEM_ACCOUNTS, change),
            Some(false)
        );
    }
    // A folder made under `C:\`: the signed-in accounts may modify it.
    let modify = "x\nD:AI(A;ID;FA;;;SY)(A;ID;FA;;;BA)(A;ID;0x1301bf;;;AU)";
    assert_eq!(
        allows_others(modify, &SYSTEM_ACCOUNTS, rights::CHANGE_FILE),
        Some(true)
    );
    // `C:\` itself lets the signed-in accounts add folders: that replaces nothing further down.
    let root =
        "x\nD:PAI(A;;FA;;;SY)(A;;FA;;;BA)(A;;0x4;;;AU)(A;OICIIO;SDGXGWGR;;;AU)(A;;0x1200a9;;;BU)";
    assert_eq!(
        allows_others(root, &SYSTEM_ACCOUNTS, rights::REPLACE_BELOW),
        Some(false)
    );
    assert_eq!(
        allows_others(root, &SYSTEM_ACCOUNTS, rights::CHANGE_FOLDER),
        Some(true)
    );
    // One person with full control, or one who may change the access list or take it over.
    for rights_given in ["FA", "WD", "WO", "SD", "0x2", "0x4", "GW", "GA", "0xzz"] {
        let list = format!("x\nD:PAI(A;;FA;;;SY)(A;;{rights_given};;;{PERSON})");
        assert_eq!(
            allows_others(&list, &SYSTEM_ACCOUNTS, rights::CHANGE_FILE),
            Some(true),
            "{rights_given}"
        );
    }
    // The person is trusted when the caller says so (the account that runs the program).
    let own = format!("x\nD:PAI(A;;FA;;;SY)(A;;FA;;;{PERSON})");
    let trusted: Vec<&str> = SYSTEM_ACCOUNTS.iter().copied().chain([PERSON]).collect();
    assert_eq!(
        allows_others(&own, &trusted, rights::CHANGE_FILE),
        Some(false)
    );
    // No list is no answer.
    assert_eq!(
        allows_others("", &SYSTEM_ACCOUNTS, rights::CHANGE_FILE),
        None
    );
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
