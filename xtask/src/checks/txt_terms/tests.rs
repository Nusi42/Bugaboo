//! The tests of TXT-TERMS.

use std::path::{Path, PathBuf};

use super::*;
use crate::catalog::Catalog;

/// The findings for `sources`, as a run of the whole repository gives them.
fn evaluate(
    terms: &[Term],
    sources: &[Source],
    exemptions: &Exemptions,
) -> anyhow::Result<Vec<Finding>> {
    evaluate_with(terms, sources, exemptions, Coverage::Whole)
}

const GLOSSARY_TEXT: &str = "\
| Say | Meaning | Don't say |
|-----|---------|-----------|
| **Sign in** | Open your use | log in, login, logout |
| **Group** | A set of people | team (as object) |
| **Search language** | The text you type | any \"xQL\" name |
| **Second step** | The check after the password | MFA, 2FA, TOTP (in UI) |
| **Status group** | One of three | status category, To Do |
| **Issue** | Anything tracked | ticket, work item |
| **Sub-issue** | An issue under another | sub-task, custom field (UI), attribute |
| **Page** | A document | — |
| **Assigned to** | Who works on it | assignee (in UI and code) |
| **Create** | Bring something new | made (in UI), making (in UI), make, add new (in UI and code) |
| **Change many issues** | Change several together | bulk, mass update (in UI and code) |

| Term | Meaning |
|------|---------|
| Instance | One server |
";

fn terms() -> Vec<Term> {
    banned_terms(GLOSSARY_TEXT)
}

fn written(terms: &[Term]) -> Vec<&str> {
    terms.iter().map(|term| term.written.as_str()).collect()
}

fn text(page: &str) -> Source {
    Source {
        place: Place {
            key: "page.md".to_owned(),
            shown: "page.md".to_owned(),
        },
        kind: Kind::Text,
        text: page.to_owned(),
    }
}

fn hits(page: &str) -> Vec<String> {
    evaluate(&terms(), &[text(page)], &Exemptions::default())
        .unwrap()
        .into_iter()
        .map(|finding| finding.message)
        .collect()
}

#[test]
fn txt_terms_reads_the_dont_say_column_of_every_table() {
    let terms = terms();
    assert_eq!(
        written(&terms),
        [
            "log in",
            "login",
            "logout",
            "MFA",
            "2FA",
            "TOTP",
            "status category",
            "To Do",
            "ticket",
            "work item",
            "sub-task",
            "custom field",
            "attribute",
            "assignee",
            "made",
            "making",
            "make",
            "add new",
            "bulk",
            "mass update"
        ]
    );
    assert_eq!(terms[0].say, "Sign in");
}

/// The findings for one declared name of the code.
fn code_hits(name: &str) -> Vec<String> {
    let source = Source {
        place: Place {
            key: format!("crates/x/src/lib.rs#fn {name}"),
            shown: format!("crates/x/src/lib.rs: fn {name}"),
        },
        kind: Kind::Code,
        text: name.to_owned(),
    };
    evaluate(&terms(), &[source], &Exemptions::default())
        .unwrap()
        .into_iter()
        .map(|finding| finding.message)
        .collect()
}

#[test]
fn txt_terms_a_code_name_with_a_word_that_asks_for_code_fails_with_the_row_of_the_glossary() {
    // A Rust type, a TypeScript function and a message key, read as the words they are made of.
    let declared = |names: Vec<code_names::Declared>| {
        names
            .into_iter()
            .flat_map(|declared| code_hits(&declared.name))
            .collect::<Vec<_>>()
    };
    let rust = declared(code_names::rust_names("pub struct IssueAssignee;").unwrap());
    assert_eq!(rust.len(), 1, "{rust:?}");
    assert!(rust[0].contains("\"assignee\"") && rust[0].contains("Say \"Assigned to\""));
    let web = declared(code_names::web_names("export function makeProject() {}"));
    assert_eq!(web.len(), 1, "{web:?}");
    assert!(web[0].contains("\"make\"") && web[0].contains("Say \"Create\""));
    for name in [
        "many_change_run",
        "ManyChangeRun",
        "create_project",
        "readable_id",
    ] {
        assert!(code_hits(name).is_empty(), "{name}");
    }
    assert_eq!(code_hits("apply_bulk").len(), 1);
    assert_eq!(code_hits("AddNewIssue").len(), 1);
}

#[test]
fn txt_terms_a_name_from_other_peoples_code_is_not_read() {
    // The standard library names `make_ascii_lowercase`; this code only calls it.
    let names = code_names::rust_names(
        "fn lower(text: &mut String) { text.make_ascii_lowercase(); }
         impl std::io::Write for Sink { fn make_writer(&self) {} }",
    )
    .unwrap();
    let found: Vec<String> = names
        .into_iter()
        .flat_map(|declared| code_hits(&declared.name))
        .collect();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn txt_terms_made_and_making_fail_in_texts_and_are_right_in_code_names() {
    assert_eq!(hits("The group is made.").len(), 1);
    assert_eq!(hits("Making your own account is off.").len(), 1);
    assert!(hits("Created the group.").is_empty());
    assert!(code_hits("made_at").is_empty());
    assert_eq!(code_hits("make_group").len(), 1);
}

#[test]
fn txt_terms_the_ordinary_words_of_programming_are_right_in_code() {
    // `ticket` is on the list for what people read, but its entry does not ask for code names.
    assert!(code_hits("ticket_id").is_empty());
    assert!(code_hits("TicketBody").is_empty());
    assert_eq!(hits("Open a ticket.").len(), 1);
}

#[test]
fn txt_terms_a_route_is_read_as_a_name_for_every_entry() {
    let name = |text: &str| Source {
        place: Place {
            key: format!("api:path {text}"),
            shown: format!("api: path {text}"),
        },
        kind: Kind::Name,
        text: text.to_owned(),
    };
    let found = |text: &str| {
        evaluate(&terms(), &[name(text)], &Exemptions::default())
            .unwrap()
            .into_iter()
            .map(|finding| finding.message)
            .collect::<Vec<_>>()
    };
    let bulk = found("/api/v1/bulk/x");
    assert_eq!(bulk.len(), 1, "{bulk:?}");
    assert!(bulk[0].contains("Say \"Change many issues\""));
    assert!(found("/api/v1/many-changes/x").is_empty());
}

#[test]
fn txt_terms_the_entries_that_ask_for_code_are_the_ones_with_code_in_their_bracket() {
    let terms = terms();
    let asks = |word: &str| {
        terms
            .iter()
            .find(|term| term.written == word)
            .map(|term| term.code)
    };
    assert_eq!(asks("assignee"), Some(true));
    assert_eq!(asks("ticket"), Some(false));
    assert_eq!(asks("MFA"), Some(false));
}

#[test]
fn txt_terms_leaves_the_entries_of_one_sense_to_review() {
    // `team (as object)` and `any "xQL" name` cannot be told from ordinary use.
    let terms = terms();
    let names = written(&terms);
    assert!(!names.contains(&"team"));
    assert!(!names.iter().any(|name| name.contains("xQL")));
}

#[test]
fn txt_terms_a_word_inside_a_longer_word_does_not_count() {
    assert_eq!(hits("Please login.").len(), 1);
    assert_eq!(hits("Please Login now.").len(), 1);
    assert!(hits("Blogging is fun, and so is logins.").is_empty());
    assert!(hits("A ticketing system.").is_empty());
}

#[test]
fn txt_terms_a_phrase_matches_with_a_space_or_a_hyphen() {
    assert_eq!(hits("You can log in here.").len(), 1);
    assert_eq!(hits("You can log-in here.").len(), 1);
    assert_eq!(hits("You can log\nin here.").len(), 1);
    assert_eq!(hits("A sub task and a sub-task.").len(), 1);
    assert!(hits("Log, then in the morning.").is_empty());
}

#[test]
fn txt_terms_an_entry_with_a_capital_letter_is_matched_as_written() {
    assert_eq!(hits("The status To Do is gone.").len(), 1);
    assert!(hits("You have to do that.").is_empty());
    assert_eq!(hits("Turn on MFA.").len(), 1);
    assert!(hits("Turn on mfa.").is_empty());
}

#[test]
fn txt_terms_a_name_in_a_setting_or_a_placeholder_is_not_text() {
    assert!(hits("Set log_in_url first.").is_empty());
    let open = readable::message_text("Open {ticket} now");
    assert!(open.contains("Open"));
    assert!(hits(&open).is_empty());
    // The text inside a plural stays.
    let plural = readable::message_text("{count, plural, one {# ticket} other {# tickets}}");
    assert_eq!(hits(&plural).len(), 1);
    // The keywords of a plural or a select are code, not words.
    let select = readable::message_text("{kind, select, ticket {Sorted} other {Plain}} list");
    assert!(hits(&select).is_empty(), "{select}");
    assert!(select.contains("Sorted") && select.contains("list"));
    let nested =
        readable::message_text("{n, plural, =0 {none} one {one {kind} login} other {many}}");
    assert_eq!(hits(&nested).len(), 1, "{nested}");
    // A brace that is never closed is plain text.
    assert!(readable::message_text("a { b").contains("a { b"));
}

#[test]
fn txt_terms_pages_are_read_without_code() {
    let page = "A `login` span and [a link](https://example.test/login) and
```
login
```
plain login";
    let prose = readable::prose(page);
    assert_eq!(hits(&prose).len(), 1);
    assert!(prose.contains("plain login"));
    assert!(!prose.contains("example.test"));
    // Tilde fences and indented code are code too, and the lines keep their numbers.
    let more = "intro

~~~
login
~~~

    login
    login

plain login
";
    let prose = readable::prose(more);
    assert_eq!(prose.lines().count(), more.lines().count());
    assert_eq!(hits(&prose).len(), 1, "{prose}");
    // Inside a list, four spaces continue the item, which is prose.
    let list = "- an item

    continues with login
";
    assert_eq!(hits(&readable::prose(list)).len(), 1);
    // A closed fence ends: what follows is prose again.
    assert_eq!(
        hits(&readable::prose(
            "```
x
```
login
"
        ))
        .len(),
        1
    );
}

#[test]
fn txt_terms_api_names_are_cut_into_words() {
    assert_eq!(name_words("link_path"), "link path");
    assert_eq!(name_words("/api/v1/log-in"), " api v1 log in");
    assert_eq!(name_words("TicketNumberBody"), "Ticket Number Body");
    let name = |text: &str| Source {
        place: Place {
            key: format!("api:schema {text}"),
            shown: format!("api: schema {text}"),
        },
        kind: Kind::Name,
        text: text.to_owned(),
    };
    let found = |text: &str| {
        evaluate(&terms(), &[name(text)], &Exemptions::default())
            .unwrap()
            .len()
    };
    assert_eq!(found("log_in_body"), 1);
    assert_eq!(found("LogInBody"), 1);
    assert_eq!(found("/log-in"), 1);
    assert_eq!(found("login"), 1);
    assert_eq!(found("ticket_id"), 1);
    assert_eq!(found("blogging"), 0);
    assert_eq!(found("sign_in"), 0);
}

#[test]
fn txt_terms_a_finding_says_where_and_what_to_say() {
    let findings = evaluate(&terms(), &[text("one\ntwo login")], &Exemptions::default()).unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].location.as_deref(), Some("page.md:2"));
    assert!(findings[0].message.contains("\"login\""));
    assert!(findings[0].message.contains("Sign in"));
}

fn allow(term: &str, at: &str) -> Exemptions {
    Exemptions {
        allow: vec![Allow {
            term: term.to_owned(),
            at: vec![at.to_owned()],
            phrases: Vec::new(),
            reason: "A test, with a reason that says enough.".to_owned(),
        }],
    }
}

#[test]
fn txt_terms_an_exemption_covers_its_place_and_goes_stale_without_one() {
    let sources = [text("Please login.")];
    let exempt = allow("login", "page.*");
    assert!(evaluate(&terms(), &sources, &exempt).unwrap().is_empty());
    // Another place is not covered.
    let other = allow("login", "other.md");
    let findings = evaluate(&terms(), &sources, &other).unwrap();
    assert_eq!(findings.len(), 2, "the finding and the stale exemption");
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("matches nothing"))
    );
    // An exemption of a word that is not used fails.
    let stale = allow("login", "page.md");
    let findings = evaluate(&terms(), &[text("Fine.")], &stale).unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].location.as_deref(), Some(EXEMPTIONS_PATH));
}

#[test]
fn txt_terms_an_exemption_of_a_name_in_code_is_not_stale_when_only_changed_files_were_read() {
    let exempt = allow("make", "crates/x/src/lib.rs#fn make_ascii");
    let sources = [text("Fine.")];
    // The whole repository was read: nothing matches, so the exemption is stale.
    let whole = evaluate_with(&terms(), &sources, &exempt, Coverage::Whole).unwrap();
    assert_eq!(whole.len(), 1);
    // Only the files that changed were read: the file may be one that was not read.
    let changed = evaluate_with(&terms(), &sources, &exempt, Coverage::Changed).unwrap();
    assert!(changed.is_empty());
    // An exemption of a text is stale either way, because texts are always read whole.
    let text_exempt = allow("make", "messages/en.json#key");
    let stale = evaluate_with(&terms(), &sources, &text_exempt, Coverage::Changed).unwrap();
    assert_eq!(stale.len(), 1);
}

#[test]
fn txt_terms_an_entry_with_a_slash_is_read_as_its_parts() {
    let terms = banned_terms(
        "| Say | Meaning | Don't say |
|---|---|---|
| **Lock** | x | check out / check in, none / read (in UI) |
",
    );
    let written: Vec<&str> = terms.iter().map(|term| term.written.as_str()).collect();
    // A list of single words stays one phrase: `read` alone is an ordinary word.
    assert_eq!(written, ["check out", "check in", "none / read"]);
}

#[test]
fn txt_terms_the_whole_api_description_is_read() {
    let spec: Value = serde_json::from_str(
        r#"{
              "paths": {"/api/v1/log-in": {"get": {
                "operationId": "getTicketNumber",
                "parameters": [{"name": "work_item", "in": "query"}]
              }}},
              "components": {"schemas": {"Body": {"properties": {
                "kind": {"type": "string", "enum": ["login", "sign_in"]},
                "list": {"type": "array", "items": {"properties": {"ticket": {}}}}
              }}}}
            }"#,
    )
    .unwrap();
    let found: Vec<(&str, String)> = readable::api_names(&spec)
        .into_iter()
        .map(|name| (name.kind, name.place))
        .collect();
    for wanted in [
        ("operation", "getTicketNumber"),
        ("parameter", "work_item"),
        ("enum", "Body login"),
        ("property", "Body.ticket"),
    ] {
        assert!(
            found.contains(&(wanted.0, wanted.1.to_owned())),
            "{wanted:?} in {found:?}"
        );
    }
    // Every one of them is checked as a name.
    let sources: Vec<Source> = readable::api_names(&spec)
        .into_iter()
        .map(|name| {
            let (key, shown) = readable::api_place(&name);
            Source {
                place: Place { key, shown },
                kind: Kind::Name,
                text: name.text,
            }
        })
        .collect();
    let findings = evaluate(&terms(), &sources, &Exemptions::default()).unwrap();
    let places: Vec<String> = findings.into_iter().filter_map(|f| f.location).collect();
    for wanted in [
        "operation getTicketNumber",
        "parameter work_item",
        "enum Body login",
    ] {
        assert!(
            places.iter().any(|place| place.ends_with(wanted)),
            "{wanted}: {places:?}"
        );
    }
}

#[test]
fn txt_terms_an_exemption_with_phrases_covers_only_those_phrases() {
    let exemptions = Exemptions {
        allow: vec![Allow {
            term: "login".to_owned(),
            at: vec!["page.md".to_owned()],
            phrases: vec!["the login page".to_owned()],
            reason: "A test, with a reason that says enough.".to_owned(),
        }],
    };
    let check = |page: &str| {
        evaluate(&terms(), &[text(page)], &exemptions)
            .unwrap()
            .len()
    };
    // Only the phrase: nothing is found and the exemption is used.
    assert_eq!(check("Open The Login Page."), 0);
    // The word anywhere else in the place is found, and the stale entry says nothing more.
    assert_eq!(check("Open the login page, then login."), 1);
    // Without the phrase the place is not covered, and the exemption is stale.
    assert_eq!(check("Please login."), 2);
}

/// The findings of the check on a repository folder, with the files the catalogue entry of the
/// real repository selects in it.
fn scan_folder(folder: PathBuf) -> Vec<Finding> {
    let real = Repo::discover().unwrap();
    let catalog = Catalog::load(&real).unwrap();
    let repo = Repo::at(folder);
    let context = Context {
        repo: &repo,
        catalog: &catalog,
        files: None,
    };
    let files = context.selected("TXT-TERMS").unwrap();
    scan_with(&repo, &files, &files, Coverage::Whole).unwrap()
}

fn fixture(side: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
        .join("tools/checks/fixtures/TXT-TERMS")
        .join(side)
}

#[test]
fn bad_example() {
    let findings = scan_folder(fixture("bad"));
    let places: Vec<String> = findings
        .iter()
        .filter_map(|finding| finding.location.clone())
        .collect();
    // One finding in each of the places that are read, also in the names of Rust and TypeScript.
    for wanted in [
        "messages/en.json: ",
        "userDocs/page.md:",
        "openapi.json: path",
        "openapi.json: schema",
        "openapi.json: property",
        "crates/demo/src/lib.rs: struct IssueAssignee",
        "crates/demo/src/lib.rs: fn make_project",
        "web/app/src/demo.ts: function makeProject",
    ] {
        assert!(
            places.iter().any(|place| place.contains(wanted)),
            "{wanted}: {places:?}"
        );
    }
}

#[test]
fn good_example() {
    let findings = scan_folder(fixture("good"));
    let shown: Vec<String> = findings
        .iter()
        .map(|finding| format!("{:?} {}", finding.location, finding.message))
        .collect();
    assert!(findings.is_empty(), "{shown:?}");
}

#[test]
fn txt_terms_the_repository_passes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let findings = scan_folder(root);
    let shown: Vec<String> = findings
        .iter()
        .map(|finding| format!("{:?} {}", finding.location, finding.message))
        .collect();
    assert!(findings.is_empty(), "{shown:#?}");
}
