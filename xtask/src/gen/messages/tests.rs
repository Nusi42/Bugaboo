//! The tests of messages.

use super::*;

#[test]
fn a_string_with_more_double_than_single_quotes_is_written_in_single_quotes() {
    assert_eq!(json_string("plain").unwrap(), "\"plain\"");
    assert_eq!(json_string("it's").unwrap(), "\"it's\"");
    assert_eq!(
        json_string("\"… is editing\"").unwrap(),
        "'\"… is editing\"'"
    );
    assert_eq!(
        json_string("\"a\" it's \"b\"").unwrap(),
        "'\"a\" it\\'s \"b\"'"
    );
    assert_eq!(json_string("\"a\\b\"").unwrap(), "'\"a\\\\b\"'");
}

#[test]
fn generated_file_is_up_to_date() {
    let repo = Repo::discover().unwrap();
    let text = std::fs::read_to_string(repo.path(SOURCE_LOCALE_PATH)).unwrap();
    for output in generate(&repo, &text).unwrap() {
        let on_disk = std::fs::read_to_string(repo.path(output.path)).unwrap();
        assert_eq!(
            crate::source::normalize(&on_disk),
            output.content,
            "{} is stale; run `cargo xtask gen messages`",
            output.path
        );
    }
}

/// The text of the generated file at `path`.
fn file<'a>(output: &'a [Generated], path: &str) -> &'a str {
    &output
        .iter()
        .find(|generated| generated.path == path)
        .unwrap_or_else(|| panic!("{path} is not generated"))
        .content
}

/// The text of every part of the browser catalog together.
fn all_parts(output: &[Generated]) -> String {
    PARTS
        .iter()
        .map(|part| file(output, part.path))
        .collect::<Vec<_>>()
        .join(
            "
",
        )
}

fn code(path: &str, text: &str) -> Source {
    (path.to_owned(), text.to_owned())
}

#[test]
fn placeholders_become_typed_arguments() {
    let repo = Repo::discover().unwrap();
    let output = generate(&repo, r#"{"home.b": "Hi {name}", "home.c": "Plain"}"#).unwrap();
    let types = file(&output, TYPES_OUTPUT);
    assert!(types.contains("\"home.b\": { name: string | number };"));
    assert!(!types.contains("\"home.c\": {"));
    assert!(
        file(&output, messages_rust::OUTPUT)
            .contains("pub fn home_b(name: impl std::fmt::Display)")
    );
}

#[test]
fn the_browser_catalog_leaves_out_what_only_the_server_shows() {
    let repo = Repo::discover().unwrap();
    let text = r#"{"error.a.b": "Oops", "config.x.y": "Server", "server.listening": "Up {address}", "home.title": "Hi"}"#;
    let output = generate(&repo, text).unwrap();
    let browser = format!("{}{}", all_parts(&output), file(&output, TYPES_OUTPUT));
    assert!(browser.contains("\"error.a.b\""));
    assert!(browser.contains("\"home.title\""));
    assert!(!browser.contains("config.x.y"));
    assert!(!browser.contains("server.listening"));
    let rust = file(&output, messages_rust::OUTPUT);
    assert!(rust.contains("CONFIG_X_Y"));
    assert!(rust.contains("server_listening"));
}

#[test]
fn a_new_area_stays_out_of_the_browser_until_it_is_listed() {
    let repo = Repo::discover().unwrap();
    let text = r#"{"brand-new.title": "New", "home.title": "Hi"}"#;
    let output = generate(&repo, text).unwrap();
    let browser = all_parts(&output);
    assert!(!browser.contains("brand-new.title"));
    assert!(browser.contains("home.title"));
    assert!(file(&output, messages_rust::OUTPUT).contains("BRAND_NEW_TITLE"));
}

#[test]
fn a_key_that_one_feature_alone_names_goes_to_the_part_of_that_feature() {
    let sources = [
        code(
            "web/app/src/features/fields/FieldsPage.tsx",
            r#"t("fields.title"); t("save.label"); t("nav.fields")"#,
        ),
        code(
            "web/app/src/features/people/List.tsx",
            r#"t("save.label"); t("people.title")"#,
        ),
        code("web/app/src/shell.tsx", r#"t("nav.fields")"#),
    ];
    let parts = parts_of(
        ["fields.title", "save.label", "nav.fields", "people.title"],
        &sources,
        &[],
    );
    assert_eq!(parts["fields.title"], "fields");
    assert_eq!(parts["people.title"], "people");
    // Two features, and a feature with code outside, both need the text on the first page.
    assert_eq!(parts["save.label"], "shared");
    assert_eq!(parts["nav.fields"], "shared");
}

#[test]
fn a_key_that_no_code_names_is_on_the_first_page() {
    let sources = [code(
        "web/app/src/features/fields/A.tsx",
        r#"t("fields.a")"#,
    )];
    let parts = parts_of(["fields.a", "error.resource.not-found"], &sources, &[]);
    assert_eq!(parts["error.resource.not-found"], "shared");
}

#[test]
fn the_mistakes_of_a_search_belong_to_the_search_and_not_to_the_first_page() {
    let sources = [code(
        "web/app/src/features/fields/A.tsx",
        r#"t("fields.a")"#,
    )];
    let keys = [
        "fields.a",
        "search.mistake.expected-value",
        "error.resource.not-found",
    ];
    let parts = parts_of(keys, &sources, &[]);
    assert_eq!(parts["search.mistake.expected-value"], "search");
    assert_eq!(parts["error.resource.not-found"], "shared");
    // A word of an enum that makes the key puts it on the first page, as for every key.
    let made = parts_of(keys, &sources, &["search.mistake.expected-value"]);
    assert_eq!(made["search.mistake.expected-value"], "shared");
}

#[test]
fn tests_and_the_generated_catalog_do_not_decide_where_a_key_goes() {
    let sources = [
        code("web/app/src/features/fields/A.tsx", r#"t("fields.a")"#),
        code("web/app/src/shared/A.test.ts", r#"t("fields.a")"#),
        code("web/app/e2e/a.spec.ts", r#"t("fields.a")"#),
        code(generated_folder!("shared.ts"), r#""fields.a": "A","#),
    ];
    assert_eq!(parts_of(["fields.a"], &sources, &[])["fields.a"], "fields");
}

#[test]
fn a_folder_that_the_list_of_features_lacks_counts_as_shared_code() {
    let sources = [code(
        "web/app/src/features/brand-new/A.tsx",
        r#"t("fields.a")"#,
    )];
    assert_eq!(parts_of(["fields.a"], &sources, &[])["fields.a"], "shared");
}

#[test]
fn a_key_that_a_word_of_an_enum_makes_is_shared_even_if_one_feature_names_it() {
    let sources = [code(
        "web/app/src/features/issues/A.tsx",
        r#"t("issue-field.title"); t("issues.list")"#,
    )];
    let parts = parts_of(
        ["issue-field.title", "issues.list"],
        &sources,
        &["issue-field.title"],
    );
    assert_eq!(parts["issue-field.title"], "shared");
    assert_eq!(parts["issues.list"], "issues");
}

#[test]
fn the_keys_of_the_enums_are_shared_in_the_repository() {
    let repo = Repo::discover().unwrap();
    let text = read_text(&repo, SOURCE_LOCALE_PATH).unwrap();
    let output = generate(&repo, &text).unwrap();
    let shared = file(&output, generated_folder!("shared.ts"));
    for made in made_keys() {
        let quoted = json_string(made.key.as_str()).unwrap();
        if !is_server_only(made.key.as_str()) {
            assert!(shared.contains(&format!("  {quoted}:")), "{}", made.key);
        }
    }
}

#[test]
fn a_template_of_shared_code_puts_every_key_it_can_build_in_shared() {
    let sources = [
        code(
            "web/app/src/shared/names.ts",
            "t(`issue-field.${code}`); const k = \"markdown.\" + name;",
        ),
        code(
            "web/app/src/features/issues/A.tsx",
            r#"t("issue-field.title")"#,
        ),
        code("web/app/src/features/people/A.tsx", r#"t("markdown.bold")"#),
    ];
    let keys = ["issue-field.title", "issue-field.priority", "markdown.bold"];
    let parts = parts_of(keys, &sources, &[]);
    assert_eq!(parts["issue-field.title"], "shared");
    assert_eq!(parts["issue-field.priority"], "shared");
    assert_eq!(parts["markdown.bold"], "shared");
}

#[test]
fn a_template_inside_one_feature_leaves_its_keys_in_that_feature() {
    let sources = [
        code(
            "web/app/src/features/fields/A.tsx",
            "t(`field-type.${kind}`); t('field-type.text')",
        ),
        code("web/app/src/features/people/A.tsx", r#"t("people.title")"#),
    ];
    let parts = parts_of(
        ["field-type.text", "field-type.date", "people.title"],
        &sources,
        &[],
    );
    assert_eq!(parts["field-type.text"], "fields");
    assert_eq!(parts["field-type.date"], "fields");
    assert_eq!(parts["people.title"], "people");
    // The same template in a second feature makes them shared.
    let more = [
        code(
            "web/app/src/features/fields/A.tsx",
            "t(`field-type.${kind}`)",
        ),
        code(
            "web/app/src/features/people/B.tsx",
            "t(`field-type.${kind}`)",
        ),
    ];
    assert_eq!(
        parts_of(["field-type.date"], &more, &[])["field-type.date"],
        "shared"
    );
}

#[test]
fn template_prefixes_are_the_text_before_the_dollar_brace_or_the_plus() {
    let found = template_prefixes(
        "t(`a.b-c.${x}`); k(\"d.\" + y); k('e.f.' +z); `${x}.no`; `plain ${x}`; \"g.h\" + 1",
    );
    assert_eq!(found, ["a.b-c.", "d.", "e.f."]);
}

#[test]
fn a_key_in_a_comment_is_not_a_use() {
    let sources = [
        code(
            "web/app/src/features/fields/A.tsx",
            "t(\"fields.a\"); // t(\"people.b\")\n/* t(`people.${c}`)\n t('people.b') */",
        ),
        code("web/app/src/features/people/A.tsx", r#"t("people.b")"#),
    ];
    let parts = parts_of(["fields.a", "people.b"], &sources, &[]);
    assert_eq!(parts["people.b"], "people");
}

#[test]
fn every_key_is_in_exactly_one_part() {
    let repo = Repo::discover().unwrap();
    let text = read_text(&repo, SOURCE_LOCALE_PATH).unwrap();
    let output = generate(&repo, &text).unwrap();
    let keys: Vec<String> = read_catalog(&text)
        .unwrap()
        .into_iter()
        .map(|(key, _)| key)
        .filter(|key| !is_server_only(key))
        .collect();
    assert!(!keys.is_empty());
    for key in keys {
        let quoted = json_string(&key).unwrap();
        let homes: Vec<&str> = PARTS
            .iter()
            .filter(|part| {
                file(&output, part.path)
                    .lines()
                    .any(|line| line.trim_start().starts_with(&format!("{quoted}:")))
            })
            .map(|part| part.name)
            .collect();
        assert_eq!(homes.len(), 1, "{key} is in {homes:?}");
    }
}

#[test]
fn there_is_a_part_for_every_feature_folder_and_for_nothing_else() {
    let repo = Repo::discover().unwrap();
    let mut folders: Vec<String> = std::fs::read_dir(repo.path(FEATURES_FOLDER))
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    folders.push(SHARED.to_owned());
    folders.sort();
    let parts: Vec<&str> = PARTS.iter().map(|part| part.name).collect();
    assert_eq!(
        folders, parts,
        "PARTS must list `shared` and every folder of {FEATURES_FOLDER}, in this order"
    );
}

#[test]
fn every_part_has_its_file_in_the_generated_folder() {
    for part in &PARTS {
        assert_eq!(
            part.path,
            format!("{GENERATED_FOLDER}{}.ts", part.name),
            "{}",
            part.name
        );
    }
}

#[test]
fn duplicate_keys_are_rejected() {
    assert!(generate(&Repo::discover().unwrap(), r#"{"a.b": "x", "a.b": "y"}"#).is_err());
}
