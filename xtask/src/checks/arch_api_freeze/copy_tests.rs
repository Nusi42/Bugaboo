//! The tests of the copy that the frozen copy must hold, in throw-away repositories.

use serde_json::{Value, json};

use super::*;
use crate::testutil::TempDir;

const SPEC: &str = "web/packages/api-client/openapi.json";

fn description(extra: bool) -> Value {
    let mut schema = json!({"type": "object", "properties": {"id": {"type": "string"}}});
    if extra {
        schema["properties"]["title"] = json!({"type": "string"});
    }
    json!({"paths": {}, "components": {"schemas": {"Thing": schema}}})
}

/// A repository with `main` that holds `kept` as its frozen copy (or none), and a branch that
/// holds the description `now`.
fn repo(name: &str, kept: Option<&Value>, now: &Value) -> (TempDir, Repo) {
    let temp = TempDir::new(name);
    assert!(temp.git(&["init", "--quiet", "-b", "main"]));
    temp.write("README", "x");
    if let Some(kept) = kept {
        temp.write(FROZEN_PATH, serde_json::to_string(kept).unwrap());
    }
    assert!(temp.git(&["add", "--force", "."]));
    assert!(temp.git(&["commit", "--quiet", "-m", "chore: main"]));
    assert!(temp.git(&["checkout", "--quiet", "-b", "work"]));
    temp.write(SPEC, serde_json::to_string(now).unwrap());
    let repo = Repo::at(temp.path().to_path_buf());
    (temp, repo)
}

#[test]
fn good_example_there_is_nothing_to_keep_before_main_has_a_copy() {
    let (_temp, repo) = repo("freeze-none", None, &description(true));
    assert!(copy_to_keep(&repo).unwrap().is_none());
    write_frozen(&repo).unwrap();
    assert!(repo.path(FROZEN_PATH).exists());
}

#[test]
fn good_example_a_new_copy_may_hold_more_than_the_copy_at_the_merge_base() {
    let (_temp, repo) = repo("freeze-more", Some(&description(false)), &description(true));
    let (origin, _) = copy_to_keep(&repo).unwrap().unwrap();
    assert!(origin.contains("merge base"), "{origin}");
    write_frozen(&repo).unwrap();
}

#[test]
fn bad_example_a_new_copy_that_lacks_a_fact_of_the_copy_to_keep_is_refused() {
    let (_temp, repo) = repo("freeze-less", Some(&description(true)), &description(false));
    let error = write_frozen(&repo).unwrap_err();
    assert!(format!("{error:#}").contains("title"), "{error:#}");
    // And the check refuses a frozen copy in the work tree that lacks it.
    let kept = description(true);
    let frozen = description(false);
    let findings = lost_in_the_copy("at the merge base", &kept, &frozen);
    assert_eq!(findings.len(), 1);
    assert!(lost_in_the_copy("x", &frozen, &kept).is_empty());
}

#[test]
fn bad_example_a_tag_without_a_copy_and_a_folder_that_is_no_repository_fail_closed() {
    let (temp, repo) = repo("freeze-tag", None, &description(true));
    // The tag is there and the commit it names has no frozen copy: an error, not a skip.
    assert!(temp.git(&["tag", FREEZE_TAG, "main"]));
    let error = copy_to_keep(&repo).unwrap_err();
    assert!(
        format!("{error:#}").contains("has no frozen copy"),
        "{error:#}"
    );
    assert!(write_frozen(&repo).is_err());
    // A git that fails is an error too.
    assert!(git(&repo, &["no-such-subcommand-for-test"]).is_err());
}

#[test]
fn good_example_the_tag_wins_over_main() {
    let (temp, repo) = repo(
        "freeze-tag-wins",
        Some(&description(false)),
        &description(true),
    );
    // The tag holds a copy with the title; main's holds less. A copy without the title is refused.
    temp.write(
        FROZEN_PATH,
        serde_json::to_string(&description(true)).unwrap(),
    );
    assert!(temp.git(&["add", "--force", "."]));
    assert!(temp.git(&["commit", "--quiet", "-m", "chore: more"]));
    assert!(temp.git(&["tag", FREEZE_TAG]));
    let (origin, kept) = copy_to_keep(&repo).unwrap().unwrap();
    assert!(origin.contains("tag"), "{origin}");
    assert!(
        broken(&kept, &description(false))
            .iter()
            .any(|line| line.contains("title"))
    );
}
