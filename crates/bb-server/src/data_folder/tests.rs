//! The tests of the data folder.

use super::*;

#[test]
fn dist_01_the_layout_is_created() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    let folder = DataFolder::prepare(&root).unwrap();
    for name in [FILES, KEYS, BACKUPS, TLS, LOGS] {
        assert!(root.join(name).is_dir(), "{name}");
    }
    assert_eq!(folder.tls(), root.join("tls"));
    assert_eq!(folder.root(), root);
    // Preparing again changes nothing and does not fail.
    DataFolder::prepare(&root).unwrap();
}

#[cfg(unix)]
#[test]
fn sec_16_folders_are_for_the_owner_only_and_repaired_on_linux() {
    use std::os::unix::fs::PermissionsExt as _;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    DataFolder::prepare(&root).unwrap();
    let mode = |name: &str| {
        std::fs::metadata(root.join(name))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    };
    for name in ["", KEYS, TLS, LOGS] {
        assert_eq!(mode(name), 0o700, "{name}");
    }
    // Someone loosens the secrets; the next start makes them private again.
    let secret = root.join(KEYS).join("master.key");
    std::fs::write(&secret, "x").unwrap();
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::set_permissions(root.join(KEYS), std::fs::Permissions::from_mode(0o755)).unwrap();
    let anchor = root.join(LOGS).join("security-head.log");
    std::fs::write(&anchor, "x").unwrap();
    std::fs::set_permissions(&anchor, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::set_permissions(root.join(LOGS), std::fs::Permissions::from_mode(0o755)).unwrap();
    DataFolder::prepare(&root).unwrap();
    assert_eq!(mode(KEYS), 0o700);
    assert_eq!(mode(LOGS), 0o700);
    assert_eq!(mode("logs/security-head.log"), 0o600);
    assert_eq!(mode("keys/master.key"), 0o600);
}

#[cfg(windows)]
fn entries(path: &Path) -> usize {
    let output = std::process::Command::new("icacls")
        .arg(path)
        .output()
        .unwrap();
    // Names of groups are translated, so count the entries instead: only one account (the
    // owner) may have rights.
    String::from_utf8_lossy(&output.stdout)
        .matches(":(")
        .count()
}

#[cfg(windows)]
#[test]
fn sec_16_folders_are_for_the_owner_only_and_repaired_on_windows() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    DataFolder::prepare(&root).unwrap();
    for name in [KEYS, TLS, LOGS] {
        assert_eq!(entries(&root.join(name)), 1, "{name}");
    }
    // Someone opens the keys and logs folders again; the next start closes them.
    let logs = root.join(LOGS);
    let keys = root.join(KEYS);
    let sid = current_sid().unwrap();
    let reset = std::process::Command::new("icacls")
        .arg(&keys)
        .args(["/inheritance:e", "/grant", "*S-1-5-32-545:(OI)(CI)R"])
        .output()
        .unwrap();
    assert!(reset.status.success());
    let reset_logs = std::process::Command::new("icacls")
        .arg(&logs)
        .args(["/inheritance:e", "/grant", "*S-1-5-32-545:(OI)(CI)R"])
        .output()
        .unwrap();
    assert!(reset_logs.status.success());
    assert!(entries(&keys) > 1, "the test could not open the folder");
    assert!(
        entries(&logs) > 1,
        "the test could not open the logs folder"
    );
    DataFolder::prepare(&root).unwrap();
    assert_eq!(entries(&keys), 1, "owner {sid} only");
    assert_eq!(entries(&logs), 1, "owner {sid} only");
}

#[cfg(windows)]
#[test]
fn sec_16_the_account_is_found_by_sid() {
    assert_eq!(
        parse_sid("\"EXAMPLE-PC\\ana\",\"S-1-5-21-1-2-3-1001\"\r\n").as_deref(),
        Some("S-1-5-21-1-2-3-1001")
    );
    assert_eq!(parse_sid("nonsense"), None);
    assert_eq!(parse_sid("\"a\",\"S-1-5-21; calc\""), None);
    assert!(current_sid().unwrap().starts_with("S-1-"));
}

#[test]
fn sec_16_a_secret_file_holds_its_text_and_replaces_an_older_file() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("code.txt");
    std::fs::write(&file, "an older and longer text").unwrap();
    write_private(&file, b"new\n", Existing::Replace).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "new\n");
    assert_eq!(names(temp.path()), ["code.txt"], "a half-made file stayed");
}

/// What is in a folder, by name.
fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn sec_16_a_failed_write_leaves_no_file_and_no_leftover() {
    use std::io::Write as _;
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("code.txt");
    let failing = |file: &mut std::fs::File| -> std::io::Result<()> {
        file.write_all(b"half of a secret")?;
        Err(std::io::Error::other("the disk is full"))
    };
    // New file: nothing is left of the half-written secret, in the folder or beside it.
    let error = write_private_with(&target, Existing::Refuse, failing).unwrap_err();
    assert_eq!(error.to_string(), "the disk is full");
    assert!(names(temp.path()).is_empty(), "{:?}", names(temp.path()));
    // Replacing: the old file stays as it was.
    std::fs::write(&target, "the old file").unwrap();
    write_private_with(&target, Existing::Replace, failing).unwrap_err();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "the old file");
    assert_eq!(names(temp.path()), ["code.txt"]);
    // A file that is there is not the failed write's to remove.
    write_private_with(&target, Existing::Refuse, failing).unwrap_err();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "the old file");
}

#[test]
fn sec_16_a_secret_is_not_written_into_a_folder_that_is_not_there() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("not-there").join("code.txt");
    let error = write_private(&target, b"secret", Existing::Replace).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(
        !temp.path().join("not-there").exists(),
        "the folder was made"
    );
}

#[test]
fn sec_16_a_staging_folder_that_somebody_made_first_is_never_used() {
    let temp = tempfile::tempdir().unwrap();
    let planted = temp.path().join("planted");
    let fresh = temp.path().join("fresh");
    std::fs::create_dir(&planted).unwrap();
    std::fs::write(planted.join("theirs.txt"), "x").unwrap();
    // The first name is taken by somebody else: the next one is used, and nothing is ever
    // written into the folder that was there.
    let mut offered = vec![planted.clone(), fresh.clone()].into_iter();
    let made = create_staging_folder_with(|| Ok(offered.next().unwrap())).unwrap();
    assert_eq!(made, fresh);
    assert_eq!(names(&planted), ["theirs.txt"]);
    // Names that are all taken give up after a few tries, with an error.
    let taken = temp.path().join("taken");
    std::fs::create_dir(&taken).unwrap();
    let mut tries = 0;
    let error = create_staging_folder_with(|| {
        tries += 1;
        Ok(taken.clone())
    })
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(tries, STAGING_TRIES);
    assert!(names(&taken).is_empty(), "something was written into it");
}

#[test]
fn sec_16_a_secret_goes_to_the_new_folder_and_not_to_a_planted_one() {
    // The names are random: a planted folder with a name that was guessed does not exist,
    // and two calls never agree on a name.
    let path = Path::new("a/code.txt");
    let first = staging_name(path).unwrap();
    let second = staging_name(path).unwrap();
    assert_ne!(first, second);
    assert!(
        first.to_string_lossy().contains(".code.txt.staging-"),
        "{first:?}"
    );
}

#[cfg(windows)]
#[test]
fn sec_16_a_secret_is_made_in_a_folder_only_its_owner_can_enter() {
    // The file is first written in a folder of its own that is for the owner only, and moved
    // into place after: nobody who can read the target folder can catch it half-made.
    let temp = tempfile::tempdir().unwrap();
    let staging = create_staging_folder(&temp.path().join("code.txt")).unwrap();
    let output = std::process::Command::new("icacls")
        .arg(&staging)
        .output()
        .unwrap();
    let listing = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_eq!(listing.matches(":(").count(), 1, "{listing}");
    assert!(!listing.contains("(I)"), "{listing}");
}

#[test]
fn sec_16_without_hard_links_a_new_file_is_made_in_the_target() {
    let temp = tempfile::tempdir().unwrap();
    let staged = temp.path().join("staged.txt");
    std::fs::write(&staged, "the secret").unwrap();
    let target = temp.path().join("target.txt");
    // A drive without hard links (FAT32, exFAT, some network drives) says "unsupported".
    let unsupported =
        |_: &Path, _: &Path| Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
    place(&staged, &target, Existing::Refuse, unsupported).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "the secret");
    // It still never replaces a file that is there.
    std::fs::write(&staged, "another secret").unwrap();
    let error = place(&staged, &target, Existing::Refuse, unsupported).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "the secret");
    // A link that fails because the target is there is not a drive without links.
    let taken = |_: &Path, _: &Path| Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
    let other = temp.path().join("other.txt");
    let error = place(&staged, &other, Existing::Refuse, taken).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(!other.exists(), "the fallback ran for a taken name");
}

#[test]
fn sec_16_a_secret_file_can_refuse_to_replace_a_file() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("code.txt");
    write_private(&file, b"first", Existing::Refuse).unwrap();
    let error = write_private(&file, b"second", Existing::Refuse).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "first");
}

#[cfg(unix)]
#[test]
fn sec_16_a_secret_file_is_for_its_owner_only_on_linux() {
    use std::os::unix::fs::PermissionsExt as _;
    let temp = tempfile::tempdir().unwrap();
    let mode = |file: &Path| std::fs::metadata(file).unwrap().permissions().mode() & 0o777;
    let fresh = temp.path().join("fresh.txt");
    write_private(&fresh, b"secret", Existing::Replace).unwrap();
    assert_eq!(mode(&fresh), 0o600);
    // A file that was readable by everybody is closed before the secret goes in.
    let open = temp.path().join("open.txt");
    std::fs::write(&open, "old").unwrap();
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o644)).unwrap();
    write_private(&open, b"secret", Existing::Replace).unwrap();
    assert_eq!(mode(&open), 0o600);
}

#[cfg(windows)]
#[test]
fn sec_16_a_secret_file_is_for_its_owner_and_the_administrators_on_windows() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("code.txt");
    std::fs::write(&file, "old").unwrap();
    // The file starts with the rights of its folder, which the test folder gives to more.
    write_private(&file, b"secret", Existing::Replace).unwrap();
    let output = std::process::Command::new("icacls")
        .arg(&file)
        .output()
        .unwrap();
    let listing = String::from_utf8_lossy(&output.stdout).into_owned();
    // The owner with full control and the administrators with read rights: two entries,
    // none of them inherited.
    assert_eq!(listing.matches(":(").count(), 2, "{listing}");
    assert!(!listing.contains("(I)"), "{listing}");
    assert!(listing.contains("(R)"), "{listing}");
}

#[cfg(target_os = "linux")]
#[test]
fn sec_16_the_owner_of_a_folder_is_found_by_user_number() {
    let passwd =
        "root:x:0:0:root:/root:/bin/bash\nbugaboo:x:998:998::/nonexistent:/usr/sbin/nologin\n";
    assert_eq!(name_in_passwd(passwd, 998).as_deref(), Some("bugaboo"));
    assert_eq!(name_in_passwd(passwd, 0).as_deref(), Some("root"));
    assert_eq!(name_in_passwd(passwd, 5), None);
    // A folder this test made belongs to the account that runs the test.
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(other_owner(temp.path()), None);
}

#[test]
fn sec_16_a_service_may_take_over_only_a_new_or_a_bugaboo_folder() {
    let temp = tempfile::tempdir().unwrap();
    // Not there yet, and empty: fine.
    check_for_service(&temp.path().join("new"), false).unwrap();
    let empty = temp.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    check_for_service(&empty, false).unwrap();
    // A folder with somebody's own files is refused, and nothing in it changes.
    let mine = temp.path().join("documents");
    std::fs::create_dir(&mine).unwrap();
    std::fs::write(mine.join("tax.pdf"), "x").unwrap();
    let error = check_for_service(&mine, false).unwrap_err();
    assert!(error.message().contains("does not look like"), "{error}");
    // An unrelated folder named `keys` (or with all the names of a data folder but a file
    // in one of them) is no Bugaboo folder either.
    let other = temp.path().join("other");
    std::fs::create_dir_all(other.join(KEYS)).unwrap();
    std::fs::write(other.join(KEYS).join("id_ed25519"), "x").unwrap();
    std::fs::write(other.join("notes.txt"), "x").unwrap();
    assert!(
        check_for_service(&other, false).is_err(),
        "a folder with a `keys` folder"
    );
    let layout = temp.path().join("layout");
    DataFolder::prepare(&layout).unwrap();
    std::fs::write(layout.join(FILES).join("photo.jpg"), "x").unwrap();
    std::fs::write(layout.join("notes.txt"), "x").unwrap();
    assert!(
        check_for_service(&layout, false).is_err(),
        "a layout with somebody's file in it"
    );
    // A data folder that Bugaboo made and has not filled yet: all of its folders, all empty.
    let fresh = temp.path().join("fresh");
    DataFolder::prepare(&fresh).unwrap();
    check_for_service(&fresh, false).unwrap();
    // One that has run holds the master key or the database, also with files added later.
    let used = temp.path().join("used");
    DataFolder::prepare(&used).unwrap();
    std::fs::write(used.join(KEYS).join(bb_store::MASTER_KEY_FILE), "x").unwrap();
    std::fs::write(used.join("notes.txt"), "x").unwrap();
    check_for_service(&used, false).unwrap();
    let with_database = temp.path().join("with-database");
    DataFolder::prepare(&with_database).unwrap();
    std::fs::write(with_database.join(DATABASE_FILE), "x").unwrap();
    std::fs::write(with_database.join("notes.txt"), "x").unwrap();
    check_for_service(&with_database, false).unwrap();
    // The known files are the database and its companions and the setup code.
    std::fs::write(with_database.join(format!("{DATABASE_FILE}-wal")), "x").unwrap();
    let known = DataFolder::existing(&with_database).unwrap().known_files();
    let names: Vec<_> = known
        .iter()
        .filter_map(|file| file.file_name()?.to_str())
        .collect();
    assert_eq!(names, ["bugaboo.db", "bugaboo.db-wal"]);
}

#[cfg(unix)]
#[test]
fn sec_16_a_link_is_not_a_known_file_and_a_linked_sub_folder_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    DataFolder::prepare(&data).unwrap();
    let elsewhere = temp.path().join("elsewhere.txt");
    std::fs::write(&elsewhere, "x").unwrap();
    // The setup code file is a link to somebody else's file: it is left alone.
    std::os::unix::fs::symlink(&elsewhere, data.join(SETUP_CODE_FILE)).unwrap();
    std::fs::write(data.join(DATABASE_FILE), "x").unwrap();
    let known = DataFolder::existing(&data).unwrap().known_files();
    assert_eq!(known, [data.join(DATABASE_FILE)]);
    // A sub-folder that is a link to another place is refused for the service.
    let other = temp.path().join("other");
    std::fs::create_dir(&other).unwrap();
    std::fs::remove_dir(data.join(TLS)).unwrap();
    std::os::unix::fs::symlink(&other, data.join(TLS)).unwrap();
    let error = check_for_service(&data, false).unwrap_err();
    assert!(error.message().contains("does not look like"), "{error}");
}

#[test]
fn dist_01_a_blocked_path_is_explained() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("a-file");
    std::fs::write(&file, "x").unwrap();
    let error = DataFolder::prepare(&file.join("data")).unwrap_err();
    assert!(error.message().contains("could not be prepared"), "{error}");
}
/// A data folder whose only content is the person's own certificate in `tls/`.
fn with_own_certificate(folder: &Path) {
    std::fs::create_dir_all(folder.join(TLS)).unwrap();
    std::fs::write(folder.join(TLS).join("certificate.pem"), "x").unwrap();
    std::fs::write(folder.join(TLS).join("private-key.pem"), "x").unwrap();
}

#[test]
fn ops_03_a_data_folder_with_own_certificates_is_taken_over_only_with_the_flag() {
    let temp = tempfile::tempdir().unwrap();
    let mine = temp.path().join("mine");
    with_own_certificate(&mine);
    // The sub-folders that are empty do not matter.
    std::fs::create_dir(mine.join(KEYS)).unwrap();
    std::fs::create_dir(mine.join(FILES)).unwrap();

    // Without the flag the folder is refused, and the message names the flag.
    let error = check_for_service(&mine, false).unwrap_err();
    assert!(
        error.message().contains("--take-over-data-folder"),
        "{error}"
    );
    assert!(error.message().contains("tls"), "{error}");
    // With the flag it is taken over; nothing in it is changed by the check.
    check_for_service(&mine, true).unwrap();
    assert!(mine.join(TLS).join("private-key.pem").is_file());

    // The flag takes over nothing else: any other content of the folder or of the
    // sub-folders, a folder inside `tls`, and links are still refused, and the message of
    // those does not offer the flag.
    let refused = |name: &str, put: &dyn Fn(&Path)| {
        let folder = temp.path().join(name);
        with_own_certificate(&folder);
        put(&folder);
        let error = check_for_service(&folder, true).unwrap_err();
        assert!(
            !error.message().contains("--take-over-data-folder"),
            "{name}: {error}"
        );
        assert!(check_for_service(&folder, false).is_err(), "{name}");
    };
    refused("with-notes", &|folder| {
        std::fs::write(folder.join("notes.txt"), "x").unwrap();
    });
    refused("with-attachment", &|folder| {
        std::fs::create_dir_all(folder.join(FILES)).unwrap();
        std::fs::write(folder.join(FILES).join("photo.jpg"), "x").unwrap();
    });
    refused("with-key", &|folder| {
        std::fs::create_dir_all(folder.join(KEYS)).unwrap();
        std::fs::write(folder.join(KEYS).join("id_ed25519"), "x").unwrap();
    });
    refused("with-folder-in-tls", &|folder| {
        std::fs::create_dir_all(folder.join(TLS).join("old")).unwrap();
    });
    refused("with-other-folder", &|folder| {
        std::fs::create_dir(folder.join("documents")).unwrap();
    });
    #[cfg(unix)]
    refused("with-link-in-tls", &|folder| {
        std::os::unix::fs::symlink("/etc/passwd", folder.join(TLS).join("link.pem")).unwrap();
    });
    #[cfg(unix)]
    refused("with-linked-tls", &|folder| {
        std::fs::remove_dir_all(folder.join(TLS)).unwrap();
        std::os::unix::fs::symlink(folder, folder.join(TLS)).unwrap();
    });

    #[cfg(unix)]
    refused("with-second-name-in-tls", &|folder| {
        // A file that is also in another place under another name.
        let elsewhere = folder.parent().unwrap().join("somebody-elses.pem");
        std::fs::write(&elsewhere, "x").unwrap();
        std::fs::hard_link(&elsewhere, folder.join(TLS).join("linked.pem")).unwrap();
    });

    // A folder that has no files of its own in `tls` is as before, flag or not.
    let empty = temp.path().join("only-folders");
    DataFolder::prepare(&empty).unwrap();
    check_for_service(&empty, false).unwrap();
    check_for_service(&empty, true).unwrap();
}

#[test]
fn ops_03_an_entry_that_cannot_be_read_is_not_skipped() {
    let items: Vec<Result<u8, std::io::Error>> = vec![
        Ok(1),
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
        Ok(3),
    ];
    assert_eq!(all_ok(items.into_iter()), None);
    assert_eq!(
        all_ok(vec![Ok::<_, std::io::Error>(1), Ok(2)].into_iter()),
        Some(vec![1, 2])
    );
}
