//! The tests of sec_private.

use super::*;
use crate::catalog::Catalog;
use crate::testutil::TempDir;

/// The made-up machine of the examples.
fn machine() -> PrivateWords {
    let variables = |name: &str| match name {
        "USERNAME" => Some("Zanzibar".to_owned()),
        "COMPUTERNAME" => Some("QUUX-STATION".to_owned()),
        "USER" => Some("ab".to_owned()),
        "HOSTNAME" => Some("Admin".to_owned()),
        _ => None,
    };
    PrivateWords::from_sources(variables, Some("# mine\n\nQuuxcorp\nMüller\n"))
}

fn fixture(kind: &str, name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(FIXTURES_FOLDER)
        .join(SecPrivate::ID)
        .join(kind)
        .join(name);
    crate::source::normalize(&fs::read_to_string(path).unwrap())
}

fn rules(text: &str, in_message: bool) -> Vec<Rule> {
    let mut found: Vec<Rule> = scan_text(text, &machine(), in_message, true)
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    found.dedup();
    found
}

fn has_word(words: &PrivateWords, text: &str) -> bool {
    !words.word_spans(text).is_empty()
}

/// Every line of the fixture breaks exactly the one rule.
fn assert_every_line_breaks(name: &str, rule: Rule) {
    let text = fixture("bad", name);
    for (number, line) in text.lines().enumerate() {
        let hits = rules(line, false);
        assert_eq!(hits, [rule], "{name}, line {}", number + 1);
    }
}

#[test]
fn bad_example_profile_paths() {
    assert_every_line_breaks("profile-paths.txt", Rule::ProfilePath);
}

#[test]
fn bad_example_session_ids() {
    assert_every_line_breaks("session-ids.txt", Rule::SessionId);
}

#[test]
fn bad_example_page_links() {
    assert_every_line_breaks("page-links.txt", Rule::PageLink);
}

#[test]
fn bad_example_private_words() {
    assert_every_line_breaks("private-words.txt", Rule::PrivateWord);
}

#[test]
fn bad_example_a_commit_message_with_a_session_trailer() {
    let text = fixture("bad", "commit-message.txt");
    assert_eq!(rules(&text, true), [Rule::SessionTrailer]);
    // The trailer is a rule of messages: in a file it is only text.
    assert!(rules(&text, false).is_empty());
}

#[test]
fn good_example_allowed_forms_and_made_up_names() {
    let text = fixture("good", "allowed.txt");
    let hits = scan_text(&text, &machine(), false, true);
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn good_example_a_commit_message_without_private_things() {
    let text = fixture("good", "commit-message.txt");
    assert!(scan_text(&text, &machine(), true, true).is_empty());
}

#[test]
fn a_word_ends_where_a_letter_ends() {
    let words = machine();
    for found in [
        "hello zanzibar!",
        "on quux-station.",
        "a quuxcorp b",
        "zanzibar_notes",
        "x_zanzibar",
        "zanzibar2",
        "zanzibar42@mail",
        "ZANZIBAR",
        "MÜLLER and müller",
    ] {
        assert!(has_word(&words, found), "{found}");
    }
    // `ab` is too short and `Admin` is generic: neither is a word.
    assert!(!has_word(&words, "ab admin"));
    // Whole words only: a letter before or after continues the word.
    for missed in ["zanzibarish", "azanzibar", "my quuxcorps"] {
        assert!(!has_word(&words, missed), "{missed}");
    }
}

#[test]
fn the_account_of_the_machine_is_never_a_placeholder_in_a_path() {
    let words = machine();
    let path = |name: &str| format!("{}/{name}/work", "/home");
    // `ab` and `admin` are too short or generic to be words, but in a path they name the owner.
    for own in ["ab", "admin", "zanzibar"] {
        assert!(has_profile_path(&path(own), &words), "{own}");
    }
    assert!(!has_profile_path(&path("runner"), &words));
    assert!(!has_profile_path(&path("runner"), &PrivateWords::default()));
}

#[test]
fn the_name_and_the_text_of_a_file_are_read_in_any_encoding() {
    let temp = TempDir::new("private-names");
    temp.write("notes.txt", "fine\nowner is Zanzibar\n");
    temp.write("zanzibar-notes.md", "fine\n");
    let utf16: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain("zanzibar\r\n".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    temp.write("wide.txt", utf16);
    // Windows-1252, as German Excel writes it: `Müller` is a word of the private list.
    temp.write("latin.txt", b"M\xFCller lives here\n");
    // UTF-16 without a mark, and a file that is no text with a name inside.
    let bare: Vec<u8> = "zanzibar\r\n"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    temp.write("bare.txt", bare.clone());
    let mut binary = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    binary.extend(&bare);
    temp.write("picture.png", binary);
    let files: Vec<String> = [
        "notes.txt",
        "zanzibar-notes.md",
        "wide.txt",
        "latin.txt",
        "bare.txt",
        "picture.png",
    ]
    .map(str::to_owned)
    .into();
    let findings = scan_files(temp.path(), &files, &machine(), &[]);
    let places: Vec<_> = findings
        .iter()
        .filter_map(|finding| finding.location.clone())
        .collect();
    // A name that holds the word is shown masked, in its own finding and in the others.
    assert_eq!(
        places,
        [
            "notes.txt:2",
            "***-notes.md (its name)",
            "wide.txt:1",
            "latin.txt:1",
            "bare.txt:1",
            "picture.png (a file that is no text)",
        ]
        .map(str::to_owned)
    );
    assert_nothing_private_is_said(&findings);
}

/// No finding holds one of the words of the made-up machine.
fn assert_nothing_private_is_said(findings: &[Finding]) {
    for finding in findings {
        let said = format!(
            "{} {}",
            finding.location.as_deref().unwrap_or_default(),
            finding.message
        )
        .to_lowercase();
        for word in ["zanzibar", "quux", "müller"] {
            assert!(!said.contains(word), "{said}");
        }
    }
}

#[test]
fn names_of_branches_tags_and_files_are_masked_in_every_finding() {
    let temp = repo_with_commit("private-masked", "chore: fine");
    temp.write("zanzibar-file.txt", "zanzibar\n");
    assert!(temp.git(&["branch", "quuxcorp-work"]));
    assert!(temp.git(&["tag", "-a", "zanzibar-1", "-m", "Claude-Session: made-up"]));
    let repo = Repo::at(temp.path().to_path_buf());
    let mut findings = scan_files(
        temp.path(),
        &["zanzibar-file.txt".to_owned()],
        &machine(),
        &[],
    );
    findings.extend(scan_history(&repo, &machine(), &[], "").unwrap());
    let places: Vec<_> = findings
        .iter()
        .filter_map(|finding| finding.location.clone())
        .collect();
    for wanted in [
        "***-file.txt (its name)",
        "***-file.txt:1",
        "branch ***-work (its name)",
        "tag ***-1 (its name)",
        "tag ***-1:1",
    ] {
        assert!(
            places.iter().any(|place| place.starts_with(wanted)),
            "{wanted}: {places:?}"
        );
    }
    assert_nothing_private_is_said(&findings);
}

#[test]
fn the_id_of_a_page_link_in_a_name_is_masked() {
    let name = format!("review/{}{}", PAGE_LINKS[0], "AbCdEf0123456789");
    assert_eq!(
        masked(&name, &machine()),
        format!("review/{}***", PAGE_LINKS[0])
    );
}

#[test]
fn every_message_reads_as_one_sentence_without_a_gap() {
    for rule in [
        Rule::ProfilePath,
        Rule::SessionId,
        Rule::SessionTrailer,
        Rule::PageLink,
        Rule::PrivateWord,
    ] {
        assert!(!rule.message().contains("  "), "{rule:?}");
    }
}

#[test]
fn what_a_commit_that_is_not_pushed_adds_is_read_even_if_a_later_one_removes_it() {
    let temp = repo_with_commit("private-unpushed", "chore: fine");
    let path = format!("{}:{}Users{}mira{}x", "C", "\\", "\\", "\\");
    temp.write("notes.txt", format!("fine\n{path}\n"));
    assert!(temp.git(&["add", "."]));
    assert!(temp.git(&["commit", "--quiet", "-m", "chore: add"]));
    temp.write("notes.txt", "fine\n");
    assert!(temp.git(&["commit", "--quiet", "-a", "-m", "chore: clean"]));
    // The tree is clean; the history is not.
    let places = run_check(&temp, None);
    assert_eq!(places.len(), 1, "{places:?}");
    assert!(places[0].ends_with(", notes.txt:2"), "{places:?}");
    // Once the commits are on a remote, they are the remote's business: only the new ones count.
    let remote = TempDir::new("private-remote");
    assert!(remote.git(&["init", "--quiet", "--bare"]));
    let url = remote.path().to_string_lossy().into_owned();
    assert!(temp.git(&["remote", "add", "origin", &url]));
    assert!(temp.git(&["push", "--quiet", "-u", "origin", "HEAD"]));
    assert!(run_check(&temp, None).is_empty());
    temp.write("more.txt", format!("{path}\n"));
    assert!(temp.git(&["add", "."]));
    assert!(temp.git(&["commit", "--quiet", "-m", "chore: more"]));
    let places = run_check(&temp, None);
    assert_eq!(places.len(), 2, "{places:?}");
    assert!(
        places.iter().any(|place| place.ends_with(", more.txt:1")),
        "{places:?}"
    );
}

#[test]
fn an_exempt_file_is_free_of_private_words_but_not_of_paths() {
    let temp = TempDir::new("private-exempt");
    let path = format!("{}:{}Users{}mira{}x", "C", "\\", "\\", "\\");
    temp.write("list.txt", format!("zanzibar\n{path}\n"));
    let exempt = [Exempt {
        path: "list.txt".to_owned(),
        reason: "A data file the project does not write.".to_owned(),
    }];
    let findings = scan_files(temp.path(), &["list.txt".to_owned()], &machine(), &exempt);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].location.as_deref(), Some("list.txt:2"));
}

#[test]
fn the_private_list_is_read_with_its_marks_and_a_broken_list_is_reported() {
    let temp = TempDir::new("private-list");
    let none = |_: &str| None;
    // No list: nothing to say.
    let (words, problem) = PrivateWords::from_home(Some(temp.path()), none);
    assert!(problem.is_none() && !has_word(&words, "quuxcorp"));
    // A UTF-8 mark on the first line does not hide the first word.
    temp.write(WORDS_FILE, b"\xEF\xBB\xBFQuuxcorp\n");
    let (words, problem) = PrivateWords::from_home(Some(temp.path()), none);
    assert!(problem.is_none() && has_word(&words, "quuxcorp"));
    // So does a UTF-16 list.
    let utf16: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain("Quuxcorp\r\n".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    temp.write(WORDS_FILE, utf16);
    let (words, _) = PrivateWords::from_home(Some(temp.path()), none);
    assert!(has_word(&words, "quuxcorp"));
    // A list that cannot be used is a problem, and the problem names no word.
    temp.write(WORDS_FILE, b"Quux\0corp\n");
    let (_, problem) = PrivateWords::from_home(Some(temp.path()), none);
    assert!(problem.is_some_and(|text| !text.to_lowercase().contains("quux")));
    fs::remove_file(temp.path().join(WORDS_FILE)).unwrap();
    fs::create_dir(temp.path().join(WORDS_FILE)).unwrap();
    let (_, problem) = PrivateWords::from_home(Some(temp.path()), none);
    assert!(problem.is_some());
}

/// A repository with the files the check reads, and a commit.
fn repo_with_commit(name: &str, message: &str) -> TempDir {
    let temp = TempDir::new(name);
    temp.write(EXEMPTIONS_PATH, "");
    temp.write("a.txt", "a\n");
    assert!(temp.git(&["init", "--quiet"]));
    assert!(temp.git(&["add", "."]));
    assert!(temp.git(&["commit", "--quiet", "-m", message]));
    temp
}

fn run_check(temp: &TempDir, files: Option<&[String]>) -> Vec<String> {
    let catalog = Catalog::load(&Repo::discover().unwrap()).unwrap();
    let repo = Repo::at(temp.path().to_path_buf());
    let context = Context {
        repo: &repo,
        catalog: &catalog,
        files,
    };
    SecPrivate
        .run(&context)
        .unwrap()
        .into_iter()
        .filter_map(|finding| finding.location)
        .collect()
}

#[test]
fn the_check_reads_every_message_of_every_branch_and_tag() {
    let bad = "chore: more\n\nClaude-Session: made-up";
    let temp = repo_with_commit("private-run", "chore: fine");
    assert!(run_check(&temp, None).is_empty());
    // A commit on another branch, which a push of that branch would send.
    assert!(temp.git(&["switch", "--quiet", "-c", "other"]));
    assert!(temp.git(&["commit", "--quiet", "--allow-empty", "-m", bad]));
    assert!(temp.git(&["switch", "--quiet", "-"]));
    let places = run_check(&temp, None);
    assert_eq!(places.len(), 1, "{places:?}");
    assert!(places[0].starts_with("commit ") && places[0].ends_with(":3"));
    // A staged run looks at files only.
    assert!(run_check(&temp, Some(&[])).is_empty());
    // The message of a tag.
    let temp = repo_with_commit("private-tag", "chore: fine");
    assert!(temp.git(&["tag", "-a", "v1", "-m", bad]));
    let places = run_check(&temp, None);
    assert!(
        places.iter().any(|place| place.starts_with("tag v1")),
        "{places:?}"
    );
}

/// A user-profile path with a made-up account, built so this file holds none.
fn made_up_path() -> String {
    format!("{}:{}Users{}mira{}x", "C", "\\", "\\", "\\")
}

#[test]
fn a_file_name_in_a_commit_is_read_even_if_the_file_is_renamed_later() {
    let temp = repo_with_commit("private-renamed", "chore: fine");
    let id = format!("{}_{}", "session", "01AbCdEfGhIjKlMnOpQrStUv");
    temp.write(&format!("{id}-notes.txt"), "fine\n");
    assert!(temp.git(&["add", "."]));
    assert!(temp.git(&["commit", "--quiet", "-m", "chore: add"]));
    assert!(temp.git(&["mv", &format!("{id}-notes.txt"), "notes.txt"]));
    assert!(temp.git(&["commit", "--quiet", "-m", "chore: rename"]));
    let places = run_check(&temp, None);
    assert_eq!(places.len(), 1, "{places:?}");
    assert!(
        places[0].ends_with(", session_***-notes.txt (its name)"),
        "{places:?}"
    );
}

#[test]
fn text_in_utf16_and_binary_files_of_a_commit_is_read() {
    let temp = repo_with_commit("private-binary-history", "chore: fine");
    let wide: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain(made_up_path().encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    temp.write("wide.txt", wide);
    let mut picture = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    picture.extend(made_up_path().bytes());
    picture.extend(b"\0\0");
    temp.write("picture.png", picture);
    assert!(temp.git(&["add", "."]));
    assert!(temp.git(&["commit", "--quiet", "-m", "chore: add"]));
    assert!(temp.git(&["rm", "--quiet", "wide.txt", "picture.png"]));
    assert!(temp.git(&["commit", "--quiet", "-m", "chore: remove"]));
    let places = run_check(&temp, None);
    assert_eq!(places.len(), 2, "{places:?}");
    assert!(places.iter().any(|place| place.ends_with(", wide.txt:1")));
    assert!(
        places
            .iter()
            .any(|place| place.ends_with(", picture.png (a file that is no text)"))
    );
}

#[test]
fn commits_on_other_local_branches_are_read() {
    let temp = repo_with_commit("private-other-branch", "chore: fine");
    assert!(temp.git(&["switch", "--quiet", "-c", "other"]));
    temp.write("notes.txt", made_up_path());
    assert!(temp.git(&["add", "."]));
    assert!(temp.git(&["commit", "--quiet", "-m", "chore: add"]));
    assert!(temp.git(&["switch", "--quiet", "-"]));
    let places = run_check(&temp, None);
    assert_eq!(places.len(), 1, "{places:?}");
    assert!(places[0].ends_with(", notes.txt:1"), "{places:?}");
}

#[test]
fn the_check_reads_the_examples_of_other_checks_but_not_its_own_failing_ones() {
    let temp = repo_with_commit("private-fixtures", "chore: fine");
    let path = format!("{}:{}Users{}mira{}x", "C", "\\", "\\", "\\");
    for folder in [
        "OTHER/bad",
        "OTHER/good",
        "SEC-PRIVATE/good",
        "SEC-PRIVATE/bad",
    ] {
        temp.write(&format!("{FIXTURES_FOLDER}/{folder}/p.txt"), &path);
    }
    temp.write("package-lock.json", &path);
    let places = run_check(&temp, None);
    assert_eq!(places.len(), 4, "{places:?}");
    assert!(
        places
            .iter()
            .all(|place| !place.contains("SEC-PRIVATE/bad"))
    );
    assert!(
        places
            .iter()
            .any(|place| place.starts_with("package-lock.json"))
    );
}

#[test]
fn the_message_file_is_read_up_to_the_scissors_and_comments_count() {
    let temp = TempDir::new("private-message");
    let at = |text: &str| message_problems(&temp.write("MSG", text)).unwrap();
    assert!(at("fix: a\n\nCo-Authored-By: x\n").is_empty());
    assert_eq!(at("fix: a\n\nClaude-Session: x\n").len(), 1);
    // A comment line is read, whatever character Git is set to use for them.
    assert_eq!(at("fix: a\n\n; Claude-Session : x\n").len(), 1);
    // The diff after the scissors line is not the message.
    let diff = format!(
        "fix: a\n\n# {} >8 {}\nClaude-Session: x\n",
        "-".repeat(24),
        "-".repeat(24)
    );
    assert!(at(&diff).is_empty());
    // Nor after it when Git is set to another comment character.
    assert!(at(&diff.replace("# ", "; ")).is_empty());
}

#[test]
fn the_session_line_is_removed_from_a_memory_note_and_nothing_else() {
    let temp = TempDir::new("private-memory");
    let note = temp.write(
        ".claude/memory/note.md",
        format!("---\nname: n\n{MEMORY_SESSION_KEY}: 0123456789abcdef\n---\r\nBody\n"),
    );
    assert!(strip_memory_session(&note).unwrap());
    assert_eq!(
        fs::read_to_string(&note).unwrap(),
        "---\nname: n\n---\r\nBody\n"
    );
    // A second run has nothing to do.
    assert!(!strip_memory_session(&note).unwrap());
}

#[test]
fn other_files_keep_their_text() {
    let temp = TempDir::new("private-memory-other");
    let text = format!("{MEMORY_SESSION_KEY}: 0123456789abcdef\n");
    let elsewhere = temp.write("docs/note.md", &text);
    let not_markdown = temp.write(".claude/memory/note.txt", &text);
    assert!(!strip_memory_session(&elsewhere).unwrap());
    assert!(!strip_memory_session(&not_markdown).unwrap());
    assert_eq!(fs::read_to_string(&elsewhere).unwrap(), text);
    assert!(!strip_memory_session(&temp.path().join(".claude/memory/none.md")).unwrap());
}
