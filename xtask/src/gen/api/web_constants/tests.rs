//! The tests of `web_constants`.

use super::*;

#[test]
fn the_policy_name_comes_from_the_macro_of_the_api_words() {
    let source = (
        WORDS_SOURCE.to_owned(),
        "macro_rules! trusted_types_policy {
    () => {
        \"a-policy\"
    };
}
"
        .to_owned(),
    );
    assert!(
        browser_names(&[source])
            .unwrap()
            .contains("= \"a-policy\";")
    );
    assert!(browser_names(&[]).is_err());
}

#[test]
fn the_app_paths_come_from_the_constants_of_the_core() {
    let source = (
        APP_PATHS_SOURCE.to_owned(),
        "pub const ISSUE: &str = \"/issues/$issue\";
pub const PROJECT_SETTINGS: &str = \"/p/$project/settings\";
pub const INVITATION: &str = \"/invitation\";
pub const NOTIFICATIONS: &str = \"/n\";
pub const SIGN_IN: &str = \"/in\";
"
        .to_owned(),
    );
    let written = app_paths(&[source]).unwrap();
    assert!(written.contains("issue: \"/issues/$issue\","), "{written}");
    assert!(written.contains("signIn: \"/in\","), "{written}");
    assert!(app_paths(&[]).is_err());
}

const CATALOG: &str = "const MODE_CHOICES: &[Choice] = &[
    Choice::new(\"device\", message::A),
    Choice::new(\"light\", message::B),
];
settings! {
    /// The mode.
    APPEARANCE_MODE: Text = {
        key: \"appearance.mode\",
        default: \"device\",
        levels: [Organization, Person],
        format: Format::Choices(MODE_CHOICES),
    }

    /// A switch.
    APPEARANCE_LESS_MOTION: Bool = {
        key: \"appearance.less-motion\",
        default: false,
        bounds: (5, 1440),
    }

    /// A number.
    AUTH_DAYS: Integer = {
        key: \"auth.days\",
        default: 90,
    }
}
/// Settings that exist only for tests.
pub mod for_tests {
    settings! {
        TEST_NOTE: Text = {
            key: \"test.note\",
            default: \"none\",
        }
    }
}
";

fn catalog() -> Source {
    (CATALOG_PATH.to_owned(), CATALOG.to_owned())
}

#[test]
fn the_keys_of_the_registry_become_names_for_the_browser() {
    assert_eq!(camel_case("appearance.less-motion"), "appearanceLessMotion");
    assert_eq!(camel_case("start.page"), "startPage");
    let text = setting_keys(&[catalog()]).unwrap();
    assert!(
        text.contains("appearanceMode: \"appearance.mode\","),
        "{text}"
    );
    assert!(!text.contains("test.note"), "{text}");
    assert!(setting_keys(&[]).is_err());
}

#[test]
fn the_choices_and_the_defaults_come_from_the_registry() {
    let text = setting_keys(&[catalog()]).unwrap();
    assert!(
        text.contains("appearanceMode: [\"device\", \"light\"],"),
        "{text}"
    );
    assert!(!text.contains("appearanceLessMotion: ["), "{text}");
    assert!(text.contains("appearanceMode: \"device\","), "{text}");
    assert!(text.contains("appearanceLessMotion: false,"), "{text}");
    assert!(text.contains("authDays: 90,"), "{text}");
}

#[test]
fn a_format_of_choices_that_cannot_be_read_is_an_error() {
    let broken = (
        CATALOG_PATH.to_owned(),
        CATALOG.replace(
            "Format::Choices(MODE_CHOICES)",
            "Format::Choices(MODE, EXTRA)",
        ),
    );
    assert!(setting_keys(&[broken]).is_err());
}

#[test]
fn choices_and_defaults_may_stand_in_another_file_of_the_registry() {
    let split = (
        CATALOG_PATH.to_owned(),
        "settings! {
    LOOK: Text = {
        key: \"appearance.look\",
        default: DEFAULT_LOOK,
        format: Format::Choices(LOOK_CHOICES),
    }
}
"
        .to_owned(),
    );
    let themes = (
            format!("{REGISTRY_FOLDER}themes.rs"),
            "pub(super) const LOOK_CHOICES: &[Choice] = &[Choice::new(\"day\", a), Choice::new(\"night\", b)];
pub(super) const DEFAULT_LOOK: &str = \"day\";
"
                .to_owned(),
        );
    let text = setting_keys(&[split, themes]).unwrap();
    assert!(
        text.contains("appearanceLook: [\"day\", \"night\"],"),
        "{text}"
    );
    assert!(text.contains("appearanceLook: \"day\","), "{text}");
}

#[test]
fn every_catalog_of_the_registry_is_read() {
    let second = (
            format!("{REGISTRY_FOLDER}more/catalog.rs"),
            "settings! {\n    MORE: Bool = {\n        key: \"more.note\",\n        default: true,\n    }\n}\n"
                .to_owned(),
        );
    let text = setting_keys(&[catalog(), second]).unwrap();
    assert!(text.contains("moreNote: \"more.note\","), "{text}");
    assert!(text.contains("moreNote: true,"), "{text}");
}

#[test]
fn choices_that_are_not_listed_are_an_error() {
    let broken = (
        CATALOG_PATH.to_owned(),
        CATALOG.replace("MODE_CHOICES: &[Choice]", "OTHER_CHOICES: &[Choice]"),
    );
    assert!(setting_keys(&[broken]).is_err());
}

fn error_codes_source() -> Source {
    (
        ERROR_CODE_SOURCE.to_owned(),
        "error_codes! {
    /// Something.
    NotFound => (\"resource.not_found\", 404),
    /// Lost.
    EditLockLost => (\"edit.lock_lost\", 409),
    VersionChanged => (\"edit.version_changed\", 409),
}
"
        .to_owned(),
    )
}

#[test]
fn the_refusals_that_keep_a_draft_come_from_the_list_of_the_edit_module() {
    let edit = (
            EDIT_SOURCE.to_owned(),
            "pub const DRAFT_KEPT_BY: [ErrorCode; 2] = [ErrorCode::EditLockLost, ErrorCode::VersionChanged];"
                .to_owned(),
        );
    let file = syn::parse_file(&edit.1).unwrap();
    let sources = [edit, error_codes_source()];
    assert_eq!(
        draft_kept_codes(&file, &sources).unwrap(),
        ["edit.lock_lost", "edit.version_changed"]
    );
    assert!(draft_kept_codes(&file, &sources[..1]).is_err());
    let unknown =
        syn::parse_file("pub const DRAFT_KEPT_BY: [ErrorCode; 1] = [ErrorCode::Nope];").unwrap();
    assert!(draft_kept_codes(&unknown, &sources).is_err());
    let none = syn::parse_file("pub const OTHER: u8 = 1;").unwrap();
    assert!(draft_kept_codes(&none, &sources).is_err());
}

#[test]
fn the_times_of_editing_come_from_the_edit_module() {
    let source = (
        EDIT_SOURCE.to_owned(),
        "pub const LEASE_SECONDS: i64 = 60;
pub const HEARTBEAT_SECONDS: i64 = 15;
pub const WATCH_SECONDS: i64 = 30;
pub const DRAFT_KEEP_DAYS: i64 = 30;
pub const DRAFT_TEXT_MAX: usize = 100_000;
pub const VERSIONS_MAX: u32 = 50;
pub const DRAFT_KEPT_BY: [ErrorCode; 2] = [ErrorCode::EditLockLost, ErrorCode::VersionChanged];
pub const LAYOUT_PART: &str = \"layout\";
pub const WORKFLOW_PART: &str = \"workflow\";
pub const DESCRIPTION_PART: &str = \"description\";
"
        .to_owned(),
    );
    let comment = (
        COMMENT_SOURCE.to_owned(),
        "pub const TEXT_PART: &str = \"text\";".to_owned(),
    );
    let version = (
        VERSION_SOURCE.to_owned(),
        "pub const FIRST_VERSION: i64 = 1;".to_owned(),
    );
    let time = (
        TIME_SOURCE.to_owned(),
        "pub const SECONDS_PER_MINUTE: u32 = 60;
pub const MINUTES_PER_HOUR: u32 = 60;
pub const SECONDS_PER_DAY: i64 = 86_400;"
            .to_owned(),
    );
    let text = editing(&[source, comment, version, time, error_codes_source()]).unwrap();
    assert!(text.contains("leaseSeconds = 60;"), "{text}");
    assert!(text.contains("heartbeatSeconds = 15;"), "{text}");
    assert!(text.contains("watchSeconds = 30;"), "{text}");
    assert!(text.contains("draftKeepDays = 30;"), "{text}");
    assert!(text.contains("draftTextMax = 100000;"), "{text}");
    assert!(text.contains("versionsMax = 50;"), "{text}");
    assert!(
        text.contains("draftKeptCodes = [\"edit.lock_lost\", \"edit.version_changed\"] as const;"),
        "{text}"
    );
    assert!(text.contains("layoutEditPart = \"layout\";"), "{text}");
    assert!(text.contains("workflowEditPart = \"workflow\";"), "{text}");
    assert!(
        text.contains("descriptionEditPart = \"description\";"),
        "{text}"
    );
    assert!(text.contains("commentEditPart = \"text\";"), "{text}");
    assert!(text.contains("firstVersion = 1;"), "{text}");
    assert!(text.contains("secondsPerDay = 86400;"), "{text}");
    assert!(editing(&[]).is_err());
}

#[test]
fn the_words_of_a_vocabulary_come_from_the_api_description_in_their_order() {
    let schemas = WORD_LISTS
        .iter()
        .map(|name| format!(r#""{name}":{{"type":"string","enum":["bug","flag"]}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let spec = format!(r#"{{"components":{{"schemas":{{{schemas}}}}}}}"#);
    let issues = (
        ISSUE_SOURCE.to_owned(),
        "impl Priority { pub const NEW: Self = Self::Normal; }
words! { pub enum Priority { Low => \"low\", Normal => \"normal\" } }"
            .to_owned(),
    );
    let text = words(&spec, std::slice::from_ref(&issues)).unwrap();
    assert!(text.contains("newIssuePriority = \"normal\";"), "{text}");
    assert!(
        text.contains("export const issueIconWords = [\"bug\", \"flag\"] as const;"),
        "{text}"
    );
    assert!(text.contains("export const choiceColorWords = [\"bug\", \"flag\"] as const;"));
    assert!(words("{}", &[issues]).is_err());
    assert!(words("not json", &[]).is_err());
    assert!(
        words(&spec, &[]).is_err(),
        "the source of the issue is missing"
    );
}

#[test]
fn m3_f12_the_places_of_rendered_html_follow_the_answers_of_each_operation() {
    let spec = r##"{
          "paths": {
            "/a/{id}": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/One"}}}}}}},
            "/b": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Many"}}}}}}},
            "/c": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Other"}}}}}}}
          },
          "components": {"schemas": {
            "One": {"type": "object", "properties": {"html": {"type": "string", "x-safe-html": true}}},
            "Many": {"type": "object", "properties": {"items": {"type": "array", "items": {"$ref": "#/components/schemas/Row"}}}},
            "Row": {"type": "object", "properties": {"body": {"$ref": "#/components/schemas/One"}, "next": {"$ref": "#/components/schemas/Row"}}},
            "Other": {"type": "object", "properties": {"html": {"type": "string"}}}
          }}
        }"##;
    let found = safe_html_paths(spec).unwrap();
    assert_eq!(found["GET /a/{id}"], [["html"]]);
    assert_eq!(found["GET /b"], [["items", "*", "body", "html"]]);
    // The same key without the mark is only text.
    assert!(!found.contains_key("GET /c"));
    let text = safe_html(spec).unwrap();
    assert!(text.contains("\"GET /a/{id}\": [[\"html\"]],"), "{text}");
}

#[test]
fn the_page_sizes_come_from_the_paging_module() {
    let source = (
        PAGING_SOURCE.to_owned(),
        "pub const PAGE_SIZE_MAX: u32 = 100;\npub const PAGE_SIZE_DEFAULT: u32 = 50;\n".to_owned(),
    );
    let text = paging(&[source]).unwrap();
    assert!(text.contains("pageSizeMax = 100;"), "{text}");
    assert!(text.contains("pageSizeDefault = 50;"), "{text}");
    assert!(paging(&[]).is_err());
}
