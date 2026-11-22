use super::*;

#[test]
fn ops_04_the_program_relies_on_itself_its_folder_and_every_folder_above() {
    let temp = tempfile::tempdir().unwrap();
    let program = temp.path().join("bugaboo");
    std::fs::write(&program, "x").unwrap();
    let chain = program_chain(&program);
    assert_eq!(chain[0].1, Part::File);
    assert_eq!(chain[1].1, Part::Folder);
    assert!(chain[2..].iter().all(|(_, part)| *part == Part::Above));
    // Up to the root of the drive.
    assert_eq!(chain.last().unwrap().0.parent(), None);
    let settings = settings_chain(&program);
    assert_eq!(settings.len(), 2);
    assert_eq!(settings[1].1, Part::Folder);
}

#[test]
fn ops_04_the_first_path_somebody_else_can_change_is_named_and_unknown_is_never_closed() {
    let chain: Vec<(PathBuf, Part)> = ["a/b/p", "a/b", "a", ""]
        .iter()
        .zip([Part::File, Part::Folder, Part::Above, Part::Above])
        .map(|(path, part)| (PathBuf::from(path), part))
        .collect();
    let open_at = |at: &'static str| move |path: &Path, _: Part| Some(path == Path::new(at));
    assert_eq!(
        first_changeable_by(&chain, &open_at("a")),
        Verdict::Open(PathBuf::from("a"))
    );
    assert_eq!(
        first_changeable_by(&chain, &|_, _| Some(false)),
        Verdict::Closed
    );
    // A path that cannot be read makes the answer unknown, and an open one further up wins.
    let unread_folder = |path: &Path, _: Part| (path != Path::new("a/b")).then_some(false);
    assert_eq!(
        first_changeable_by(&chain, &unread_folder),
        Verdict::Unknown(PathBuf::from("a/b"))
    );
    let unread_then_open =
        |path: &Path, _: Part| (path != Path::new("a/b")).then_some(path == Path::new(""));
    assert_eq!(
        first_changeable_by(&chain, &unread_then_open),
        Verdict::Open(PathBuf::new())
    );
}

#[test]
fn ops_04_a_path_root_or_the_account_owns_and_nobody_else_writes_is_trusted() {
    assert_eq!(trust_of(0o755, 0, None), None);
    assert_eq!(trust_of(0o700, 0, None), None);
    assert_eq!(trust_of(0o775, 0, None), Some(Untrusted::OthersCanWrite));
    assert_eq!(
        trust_of(0o754, 0, None),
        None,
        "only writing counts, not reading"
    );
    assert_eq!(trust_of(0o1777, 0, None), Some(Untrusted::OthersCanWrite));
    assert_eq!(trust_of(0o755, 1000, None), Some(Untrusted::NotRoots));
    assert_eq!(trust_of(0o755, 1000, Some(1000)), None);
    assert_eq!(trust_of(0o755, 1001, Some(1000)), Some(Untrusted::NotRoots));
    assert_eq!(
        trust_of(0o777, 1000, Some(1000)),
        Some(Untrusted::OthersCanWrite)
    );
}

#[test]
fn ops_04_a_program_in_the_data_folder_is_found_by_its_real_path() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    std::fs::create_dir_all(data.join("files")).unwrap();
    assert!(in_data_folder(&data.join("files").join("bugaboo"), &data));
    assert!(in_data_folder(
        &data.join("files").join("..").join("bugaboo"),
        &data
    ));
    assert!(!in_data_folder(&temp.path().join("bugaboo"), &data));
    let files = HostFiles {
        program: &data.join("bugaboo"),
        config: None,
        data_dir: &data,
        tls_key: None,
    };
    let looked = look(&files);
    assert_eq!(looked.rights().program, Access::Open);
    assert_eq!(looked.rights().settings_file, Access::Private);
    let told = looked.open_parts();
    assert_eq!(told.len(), 1);
    assert_eq!(told[0].0, "program");
    assert!(told[0].1.contains("bugaboo"), "{}", told[0].1);
}

/// A folder in the build folder of the tests: on Linux its folders above belong to the account
/// or to root, unlike the shared folder for temporary files, which everybody may write to.
fn own_folder() -> tempfile::TempDir {
    let beside = std::env::current_exe().unwrap();
    tempfile::tempdir_in(beside.parent().unwrap()).unwrap()
}

#[cfg(unix)]
#[test]
fn sec_16_a_settings_file_or_program_others_can_change_is_reported_at_start() {
    use std::os::unix::fs::PermissionsExt as _;
    let temp = own_folder();
    let program = temp.path().join("bugaboo");
    let config = temp.path().join("bugaboo.toml");
    let key = temp.path().join("key.pem");
    for path in [&program, &config, &key] {
        std::fs::write(path, "x").unwrap();
    }
    let mode = |path: &Path, mode: u32| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    mode(temp.path(), 0o755);
    mode(&program, 0o755);
    mode(&config, 0o600);
    mode(&key, 0o600);
    let data = temp.path().join("data");
    let files = HostFiles {
        program: &program,
        config: Some(&config),
        data_dir: &data,
        tls_key: Some(&key),
    };
    assert_eq!(look(&files).rights(), HostRights::default());
    // The group may change the program, everybody may read the settings and the key.
    mode(&program, 0o775);
    mode(&config, 0o644);
    mode(&key, 0o640);
    assert_eq!(
        look(&files).rights(),
        HostRights {
            program: Access::Open,
            settings_file: Access::Open,
            tls_key: Access::Open,
            preview_child: None,
        }
    );
    // A folder everybody may write to holds the program: refused for a service.
    mode(&program, 0o755);
    mode(temp.path(), 0o777);
    assert_eq!(
        first_changeable(&program_chain(&program), Besides::ThisAccount),
        Verdict::Open(std::fs::canonicalize(temp.path()).unwrap())
    );
    mode(temp.path(), 0o755);
}

#[cfg(windows)]
#[test]
fn sec_16_a_settings_file_or_program_others_can_change_is_reported_at_start_on_windows() {
    let temp = own_folder();
    let program = temp.path().join("bugaboo.exe");
    let config = temp.path().join("bugaboo.toml");
    for path in [&program, &config] {
        std::fs::write(path, "x").unwrap();
    }
    let data = temp.path().join("data");
    let files = HostFiles {
        program: &program,
        config: Some(&config),
        data_dir: &data,
        tls_key: None,
    };
    // The account's own folder: closed to everybody but it, the system and the administrators.
    assert_eq!(look(&files).rights(), HostRights::default());
    // For a service it is not closed: the person who owns the folder could change the program.
    assert!(matches!(
        first_changeable(&program_chain(&program), Besides::Nobody),
        Verdict::Open(_)
    ));
    // The users of the computer may change the program and read the settings.
    let grant = |path: &Path, entry: &str| {
        let output = std::process::Command::new("icacls")
            .arg(path)
            .args(["/grant", entry])
            .output()
            .unwrap();
        assert!(output.status.success());
    };
    grant(&program, "*S-1-5-32-545:M");
    grant(&config, "*S-1-5-32-545:R");
    let rights = look(&files).rights();
    assert_eq!(rights.program, Access::Open);
    assert_eq!(rights.settings_file, Access::Open);
    let told = look(&files).open_parts();
    assert!(
        told[0].1.contains(&program.display().to_string()),
        "{}",
        told[0].1
    );
}

/// Runs `cmd /c mklink /J link target`: a junction, which any account may make.
#[cfg(windows)]
fn junction(link: &Path, target: &Path) {
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[cfg(windows)]
#[test]
fn sec_16_a_program_behind_a_junction_is_judged_by_its_real_folders() {
    let temp = own_folder();
    let real = temp.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let program = real.join("bugaboo.exe");
    std::fs::write(&program, "x").unwrap();
    let link = temp.path().join("link");
    junction(&link, &real);
    // The chain is the one of the real place, written as people write it (no `\\?\`), so
    // `icacls` can read every part of it.
    let chain = program_chain(&link.join("bugaboo.exe"));
    let real_program = std::fs::canonicalize(&program).unwrap();
    assert!(chain[0].0.ends_with(r"real\bugaboo.exe"), "{chain:?}");
    assert!(
        !chain[0].0.to_string_lossy().starts_with(r"\\?\"),
        "{chain:?}"
    );
    assert_eq!(chain[0].0, real_path(&real_program));
    assert!(chain[1].0.ends_with("real"), "{chain:?}");
    // The data folder is found behind the junction too, whatever the case of the letters.
    let shouted = PathBuf::from(real.to_string_lossy().to_uppercase());
    assert!(in_data_folder(&link.join("bugaboo.exe"), &shouted));
}

#[cfg(windows)]
#[test]
fn sec_16_an_owner_of_the_program_is_read_and_counts() {
    let temp = own_folder();
    let program = temp.path().join("bugaboo.exe");
    std::fs::write(&program, "x").unwrap();
    let paths = [program.clone(), temp.path().to_path_buf()];
    let owners = crate::data_folder::rights::owners(&paths);
    // The account of the test made both, so it owns both.
    let me = crate::data_folder::current_sid().unwrap();
    assert_eq!(owners, [Some(me.clone()), Some(me)]);
    // A path that is not there has no owner that can be read.
    let gone = crate::data_folder::rights::owners(&[temp.path().join("gone")]);
    assert_eq!(gone, [None]);
}
