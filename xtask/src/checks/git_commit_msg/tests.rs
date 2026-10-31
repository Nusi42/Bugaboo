//! The tests of GIT-COMMIT-MSG: the messages in `tools/checks/fixtures/GIT-COMMIT-MSG`.

use std::fs;
use std::path::Path;

use super::*;
use crate::source::FIXTURES_FOLDER;
use crate::testutil::TempDir;

fn fixtures(kind: &str) -> Vec<(String, String)> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(FIXTURES_FOLDER)
        .join("GIT-COMMIT-MSG")
        .join(kind);
    let mut found: Vec<(String, String)> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            (
                name,
                crate::source::normalize(&fs::read_to_string(path).unwrap()),
            )
        })
        .collect();
    found.sort();
    found
}

/// What each failing example must be told, by the name of its file: a part of the one sentence
/// that names the fault.
fn expected(name: &str) -> &'static str {
    match name {
        "not-conventional" => "not a Conventional Commit",
        "unknown-type" => "not a commit type",
        "too-long" => "characters",
        "upper-case-start" => "starts in lower case",
        "past-tense" | "gerund" => "imperative",
        "determiner" => "which is not a verb",
        "third-person" | "third-person-es" => "third person",
        "no-blank-line" => "one blank line after the first line",
        "long-body-line" => "72 characters",
        "trailer-no-gap" => "blank line before the trailer",
        "full-stop" => "full stop",
        "empty-merge" => "Merge <what it brought>",
        "old-merge-form" => "`WIN3`",
        "other-trailer" | "session-trailer" | "other-co-author" => "is not allowed",
        "page-link" => "claude.ai",
        "folder-name" => "`win3`",
        "work-folder" => "`work folder`",
        "master" | "master-alone" => "`master`",
        "lead" | "lead-alone" => "`lead`",
        "fast-forward" => "`fast forward`",
        "wave" => "`wave`",
        "work-package" => "`work package`",
        "package-code" => "`WP`",
        "milestone-code" => "`M5`",
        "milestone-code-letter" => "`M2C`",
        "acronyms" => "`SSOT`",
        "owner" => "`owner`",
        "wip" => "`wip`",
        other => panic!("no expectation for the failing example {other}"),
    }
}

#[test]
fn good_example_messages() {
    for (name, message) in fixtures("good") {
        assert!(
            problems(&message).is_empty(),
            "{name}: {:?}",
            problems(&message)
        );
    }
}

#[test]
fn bad_example_messages_are_told_what_is_wrong() {
    for (name, message) in fixtures("bad") {
        let found = problems(&message);
        let wanted = expected(&name);
        assert!(
            found.iter().any(|problem| problem.contains(wanted)),
            "{name}: wanted {wanted:?} in {found:?}"
        );
    }
}

#[test]
fn only_the_co_author_trailer_of_claude_is_allowed() {
    for good in [
        "fix: add a\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n",
        "fix: add a\n\nCo-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>\n",
        // A sentence with a colon is no trailer.
        "fix: add a\n\nNote: this is a sentence.\n",
    ] {
        assert!(problems(good).is_empty(), "{good:?}");
    }
    for bad in [
        "fix: add a\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n",
        "fix: add a\n\nCo-Authored-By: Claude Opus 5.5 <someone@example.test>\n",
        "fix: add a\n\nco-authored-by: someone\n",
    ] {
        assert!(!problems(bad).is_empty(), "{bad:?}");
    }
}

#[test]
fn git_own_subjects_are_accepted_and_other_merges_need_a_description() {
    for subject in [
        "Revert \"feat(web): add board\"",
        "fixup! feat(xtask): add setup",
        "squash! fix(web): x",
        "amend! docs: y",
        "Merge the board",
        "Merge pull request #4 from x/y",
    ] {
        assert!(problems(subject).is_empty(), "{subject}");
    }
    assert!(!problems("Merged stuff").is_empty());
    assert!(!problems("Reverting things").is_empty());
    assert!(!problems("").is_empty());
}

#[test]
fn comment_lines_are_not_part_of_the_message() {
    assert_eq!(subject_of("# Please enter\nfix: a\n\nbody"), "fix: a");
}

#[test]
fn the_message_file_is_checked() {
    let temp = TempDir::new("commit-msg");
    let bad = temp.write("bad", "wip\n");
    let good = temp.write(
        "good",
        "fix(web): x\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n",
    );
    assert!(!check_file(&bad).unwrap().is_empty());
    assert!(check_file(&good).unwrap().is_empty());
    // The diff after the scissors line is not the message.
    let diff = format!(
        "fix(web): x\n\n# {} >8 {}\n+ the master\n",
        "-".repeat(24),
        "-".repeat(24)
    );
    assert!(check_file(&temp.write("diff", diff)).unwrap().is_empty());
}

#[test]
fn the_check_reads_every_commit_not_on_main_merges_included() {
    let temp = TempDir::new("commit-msg-branch");
    assert!(temp.git(&["init", "--quiet", "-b", "main"]));
    assert!(temp.git(&["commit", "--quiet", "--allow-empty", "-m", "chore: start"]));
    assert!(temp.git(&["switch", "--quiet", "-c", "work"]));
    assert!(temp.git(&[
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "feat(web): add a board"
    ]));
    assert!(temp.git(&["switch", "--quiet", "main"]));
    assert!(temp.git(&["commit", "--quiet", "--allow-empty", "-m", "chore: more"]));
    assert!(temp.git(&["switch", "--quiet", "work"]));
    assert!(temp.git(&[
        "merge",
        "--quiet",
        "--no-ff",
        "main",
        "-m",
        "Merge branch 'WIN3' (M5 WP-03 x)"
    ]));
    let repo = Repo::at(temp.path().to_path_buf());
    let messages = branch_messages(&repo);
    // The commit of the branch and the merge; the merge breaks the rules.
    assert_eq!(messages.len(), 2, "{messages:?}");
    let bad: Vec<_> = messages
        .iter()
        .filter(|(_, m)| !problems(m).is_empty())
        .collect();
    assert_eq!(bad.len(), 1, "{bad:?}");
}

#[test]
fn the_first_word_is_a_verb_in_the_imperative() {
    for good in [
        "fix(a): add setup",
        "fix(a): use one reader",
        "fix(a): focus the field",
        "fix(a): process the list",
        "fix(a): pass the token",
        "fix(a): embed the font",
        "fix(a): SEC-PRIVATE reads the range",
        "fix(a): README explains it",
        "fix(a): TypeScript checks run",
        "docs(a): news about the release",
    ] {
        assert!(problems(good).is_empty(), "{good}: {:?}", problems(good));
    }
    for bad in [
        "fix(a): the list",
        "fix(a): a button",
        "fix(a): An item",
        "fix(a): one reader",
        "fix(a): no empty card",
        "fix(a): every route",
        "fix(a): each row",
        "fix(a): all rows",
        "fix(a): this page",
        "fix(a): that page",
        "fix(a): these pages",
        "fix(a): those pages",
        "fix(a): its name",
        "fix(a): their names",
        "fix(a): some names",
        "fix(a): adds a field",
        "fix(a): fixes a field",
        "fix(a): makes a field",
        "fix(a): uses a field",
        "fix(a): removes a field",
        "fix(a): keeps a field",
        "fix(a): shows a field",
        "fix(a): moves a field",
        "fix(a): reads a field",
        "fix(a): writes a field",
        "fix(a): applies a field",
        "fix(a): Add a field",
        "fix(a): Readme explains it",
    ] {
        assert!(!problems(bad).is_empty(), "{bad}");
    }
}

#[test]
fn the_body_follows_the_common_layout() {
    let long = "x".repeat(80);
    let trailer = "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>";
    let has_72 = |message: &str| problems(message).iter().any(|p| p.contains("72"));
    assert!(has_72(&format!("fix(a): b\n\n{long}\n")));
    assert!(problems("fix(a): b\n\nshort\n").is_empty());
    // A URL, an indented line and a fenced code block may be long.
    assert!(!has_72(&format!(
        "fix(a): b\n\nhttps://example.test/{long}\n"
    )));
    assert!(!has_72(&format!("fix(a): b\n\n  {long}\n")));
    assert!(!has_72(&format!("fix(a): b\n\n```\n{long}\n```\n")));
    assert!(!problems("fix(a): b\nbody\n").is_empty());
    assert!(!problems(&format!("fix(a): b\nbody\n{trailer}\n")).is_empty());
    assert!(!problems(&format!("fix(a): b\n\nbody\n{trailer}\n")).is_empty());
    assert!(problems(&format!("fix(a): b\n\nbody\n\n{trailer}\n")).is_empty());
    assert!(problems(&format!("fix(a): b\n\n{trailer}\n")).is_empty());
    // A subject alone is fine.
    assert!(problems("fix(a): b\n").is_empty());
}

#[test]
fn product_words_pass_but_their_parts_do_not() {
    assert!(problems("docs(a): ask the project lead").is_empty());
    assert!(problems("docs(a): rotate the Master Key").is_empty());
    assert!(!problems("docs(a): ask the lead").is_empty());
    assert!(!problems("docs(a): ask the lead of the project").is_empty());
    assert!(!problems("docs(a): rotate the master").is_empty());
    assert!(!problems("docs(a): reset the master of keys").is_empty());
}
