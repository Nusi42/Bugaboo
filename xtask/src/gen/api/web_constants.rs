//! What the browser app takes from the Rust code as constants, so it never writes them a second
//! time: the keys, the choices and the defaults of the settings (ADR-0018), the page sizes of
//! the lists (PERF-02), the times and sizes of editing (the lease, the heartbeat, how long a
//! draft is kept, how many records one check of versions takes: ADR-0019), and the words of the
//! fixed vocabularies the browser has to list (the icons of issue types).
//!
//! xtask does not link the product crates, so the first two are read as Rust source and the words
//! from the API description, which holds every vocabulary of the server as a list of words.

use std::fmt::Write as _;

use anyhow::{Context as _, bail};

use super::TS_HEADER;
use crate::checks::arch_settings::{CATALOG_PATH, REGISTRY_FOLDER};
use crate::rust_registry::{ChoiceLists, read_registry};
use crate::source::Source;

const WORDS_SOURCE: &str = "crates/bb-api-words/src/lib.rs";
const POLICY_MACRO: &str = "macro_rules! trusted_types_policy";
const PAGING_SOURCE: &str = "crates/bb-core/src/paging.rs";
const EDIT_SOURCE: &str = "crates/bb-core/src/edit.rs";
const FIELDS_SOURCE: &str = "crates/bb-core/src/own_fields.rs";
const PAGE_SIZE_MAX: &str = "PAGE_SIZE_MAX";
const PAGE_SIZE_DEFAULT: &str = "PAGE_SIZE_DEFAULT";

/// `appearance.less-motion` as `appearanceLessMotion`.
fn camel_case(key: &str) -> String {
    let mut name = String::new();
    let mut up = false;
    for c in key.chars() {
        if c == '.' || c == '-' {
            up = true;
        } else if up {
            name.extend(c.to_uppercase());
            up = false;
        } else {
            name.push(c);
        }
    }
    name
}

/// The keys, the choices and the defaults of the registry as constants for the browser client. The
/// settings that exist only for tests are left out.
pub(super) fn setting_keys(sources: &[Source]) -> anyhow::Result<String> {
    let mut settings = Vec::new();
    let mut lists = ChoiceLists::new();
    let mut seen = false;
    // The other files of the registry folder hold lists and constants the catalog names, like the
    // themes that the theme generator writes (ADR-0026).
    let others: Vec<&str> = sources
        .iter()
        .filter(|(path, _)| path.starts_with(REGISTRY_FOLDER) && !path.ends_with("/catalog.rs"))
        .map(|(_, text)| text.as_str())
        .collect();
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| path.starts_with(REGISTRY_FOLDER) && path.ends_with("/catalog.rs"))
    {
        seen = true;
        let (found, choices) = read_registry(text, &others)
            .with_context(|| format!("{path} is not a settings registry"))?;
        settings.extend(found);
        lists.extend(choices);
    }
    if !seen {
        bail!("{CATALOG_PATH} is missing");
    }
    if settings.is_empty() {
        bail!("{REGISTRY_FOLDER} declares no setting");
    }
    let mut out = format!(
        "{TS_HEADER}/** The key of every setting of the registry (ADR-0018), by the name the app uses for it. */
export const settingKeys = {{
"
    );
    for setting in &settings {
        let _ = writeln!(out, "  {}: \"{}\",", camel_case(&setting.key), setting.key);
    }
    out.push_str(
        "} as const;

/** The key of a setting of the registry. */
export type SettingKey = (typeof settingKeys)[keyof typeof settingKeys];

/** The choices of every setting that holds one of a list, in the order of the registry. */
export const settingChoices = {
",
    );
    for setting in &settings {
        let Some(name) = &setting.choices else {
            continue;
        };
        let values = lists.get(name).with_context(|| {
            format!(
                "{CATALOG_PATH}: the setting `{}` names the choices `{name}`, which are not listed there",
                setting.key
            )
        })?;
        let values: Vec<String> = values.iter().map(|value| format!("{value:?}")).collect();
        let _ = writeln!(
            out,
            "  {}: [{}],",
            camel_case(&setting.key),
            values.join(", ")
        );
    }
    out.push_str(
        "} as const;

/** What a person who chose nothing gets, for every setting of the registry. */
export const settingDefaults = {
",
    );
    for setting in &settings {
        let _ = writeln!(out, "  {}: {},", camel_case(&setting.key), setting.default);
    }
    out.push_str(
        "} as const;

/** One of the choices of a setting, like `\"device\"` for `appearanceMode`. */
export type SettingChoice<Name extends keyof typeof settingChoices> =
  (typeof settingChoices)[Name][number];
",
    );
    Ok(out)
}

/// The value of `pub const NAME: u32 = 100;` in the file, which was read from `path`.
fn number_const(file: &syn::File, path: &str, name: &str) -> anyhow::Result<String> {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Const(item) if item.ident == name => match &*item.expr {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(number),
                    ..
                }) => Some(number.base10_digits().to_owned()),
                _ => None,
            },
            _ => None,
        })
        .with_context(|| format!("{path} has no `const {name}` that is a number"))
}

/// The vocabularies the browser lists or recognizes at run time (a drop-down of icons, the words of
/// a status or a priority that a value must be one of), by the name of their
/// schema in the API description. The type of each stays the generated one; only the list of words
/// is made here, so it is never written a second time.
const WORD_LISTS: [&str; 10] = [
    "IssueIcon",
    "Status",
    "StatusGroup",
    "Priority",
    "Field",
    "FieldType",
    "FieldFor",
    "Look",
    "ChoiceColor",
    "LayoutKind",
];

/// `IssueIcon` as `issueIconWords`.
fn words_name(schema: &str) -> String {
    let mut name = String::new();
    for (at, c) in schema.chars().enumerate() {
        if at == 0 {
            name.extend(c.to_lowercase());
        } else {
            name.push(c);
        }
    }
    name.push_str("Words");
    name
}

/// The words of the vocabularies in [`WORD_LISTS`], in the order of the Rust list, as constants for
/// the browser client.
pub(super) fn words(spec: &str) -> anyhow::Result<String> {
    let spec: serde_json::Value =
        serde_json::from_str(spec).context("the API description is not JSON")?;
    let mut out = String::from(TS_HEADER);
    for schema in WORD_LISTS {
        let words = spec["components"]["schemas"][schema]["enum"]
            .as_array()
            .with_context(|| format!("the API description has no list of words `{schema}`"))?;
        let words: Vec<String> = words
            .iter()
            .map(|word| {
                word.as_str()
                    .map(|text| format!("{text:?}"))
                    .with_context(|| format!("`{schema}` has a word that is not text"))
            })
            .collect::<anyhow::Result<_>>()?;
        let _ = write!(
            out,
            "/** Every word of `{schema}`, in the order the server lists them. */
export const {} = [{}] as const;

",
            words_name(schema),
            words.join(", ")
        );
    }
    Ok(out)
}

/// The mark that says a property holds HTML the server rendered.
const SAFE_HTML_MARK: &str = bb_api_words::marks::SAFE_HTML;
/// The step of a path that stands for every item of a list.
const EVERY_ITEM: &str = "*";

/// Where the properties with the mark `x-safe-html` are in the answers: for each operation
/// (`METHOD /path/{template}`), the paths of keys from the top of the answer down to the property,
/// with `*` for every item of a list (M3-F12). The client turns a string into a `SafeHtml` only at
/// these places, so a key of the same name anywhere else is only text.
pub(super) fn safe_html_paths(
    spec: &str,
) -> anyhow::Result<std::collections::BTreeMap<String, Vec<Vec<String>>>> {
    let spec: serde_json::Value =
        serde_json::from_str(spec).context("the API description is not JSON")?;
    let paths = spec["paths"]
        .as_object()
        .context("the API description has no paths")?;
    let mut found = std::collections::BTreeMap::new();
    for (address, methods) in paths {
        for (method, operation) in methods.as_object().into_iter().flatten() {
            let mut here = Vec::new();
            for (status, answer) in operation["responses"].as_object().into_iter().flatten() {
                let schema = &answer["content"]["application/json"]["schema"];
                if status.starts_with('2') && !schema.is_null() {
                    walk_for_safe_html(&spec, schema, &mut Vec::new(), &mut Vec::new(), &mut here);
                }
            }
            here.sort();
            here.dedup();
            if !here.is_empty() {
                found.insert(format!("{} {address}", method.to_uppercase()), here);
            }
        }
    }
    Ok(found)
}

/// Follows a schema: references, the parts of `allOf`/`oneOf`/`anyOf`, properties and list items.
fn walk_for_safe_html(
    spec: &serde_json::Value,
    schema: &serde_json::Value,
    path: &mut Vec<String>,
    followed: &mut Vec<String>,
    found: &mut Vec<Vec<String>>,
) {
    if let Some(reference) = schema["$ref"].as_str() {
        // A schema that refers to itself is followed once.
        if followed.iter().any(|seen| seen == reference) {
            return;
        }
        let name = reference.rsplit('/').next().unwrap_or_default();
        followed.push(reference.to_owned());
        walk_for_safe_html(
            spec,
            &spec["components"]["schemas"][name],
            path,
            followed,
            found,
        );
        followed.pop();
        return;
    }
    for keyword in ["allOf", "oneOf", "anyOf"] {
        for part in schema[keyword].as_array().into_iter().flatten() {
            walk_for_safe_html(spec, part, path, followed, found);
        }
    }
    for (name, property) in schema["properties"].as_object().into_iter().flatten() {
        path.push(name.clone());
        if property[SAFE_HTML_MARK] == serde_json::json!(true) {
            found.push(path.clone());
        } else {
            walk_for_safe_html(spec, property, path, followed, found);
        }
        path.pop();
    }
    if !schema["items"].is_null() {
        path.push(EVERY_ITEM.to_owned());
        walk_for_safe_html(spec, &schema["items"], path, followed, found);
        path.pop();
    }
}

/// The places from [`safe_html_paths`] as a constant for the browser client.
pub(super) fn safe_html(spec: &str) -> anyhow::Result<String> {
    let mut rows = String::new();
    for (operation, paths) in safe_html_paths(spec)? {
        let paths: Vec<String> = paths
            .iter()
            .map(|path| {
                let steps: Vec<String> = path.iter().map(|step| format!("{step:?}")).collect();
                format!("[{}]", steps.join(", "))
            })
            .collect();
        let _ = writeln!(rows, "  {operation:?}: [{}],", paths.join(", "));
    }
    Ok(format!(
        "{TS_HEADER}/** Where the answers of each operation hold HTML the server rendered (mark `{SAFE_HTML_MARK}`): the keys from the top of the answer down to the string, `{EVERY_ITEM}` for every item of a list. The client turns the strings there into `SafeHtml`, and no other. */
export const safeHtmlPaths: Readonly<Record<string, readonly (readonly string[])[]>> = {{
{rows}}};
"
    ))
}

/// The page sizes of the lists as constants for the browser client.
pub(super) fn paging(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == PAGING_SOURCE) else {
        bail!("{PAGING_SOURCE} is missing");
    };
    let file = syn::parse_file(text).with_context(|| format!("{PAGING_SOURCE} is not Rust"))?;
    Ok(format!(
        "{TS_HEADER}/** The most items one page of a list holds; a bigger request is cut to this. */
export const pageSizeMax = {};

/** How many items a page of a list holds when the request does not say. */
export const pageSizeDefault = {};
",
        number_const(&file, PAGING_SOURCE, PAGE_SIZE_MAX)?,
        number_const(&file, PAGING_SOURCE, PAGE_SIZE_DEFAULT)?,
    ))
}

/// The times and sizes of editing as constants for the browser client: it sends a heartbeat as
/// often as the lease needs, asks about as many records as one check takes, and names the part of
/// a layout that its edit lock is about.
pub(super) fn editing(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == EDIT_SOURCE) else {
        bail!("{EDIT_SOURCE} is missing");
    };
    let file = syn::parse_file(text).with_context(|| format!("{EDIT_SOURCE} is not Rust"))?;
    let number = |name: &str| number_const(&file, EDIT_SOURCE, name);
    let layout_part = text_const(text, "LAYOUT_PART")
        .with_context(|| format!("{EDIT_SOURCE} has no `const LAYOUT_PART` that is text"))?;
    Ok(format!(
        "{TS_HEADER}/** How long an edit lock lasts after the last heartbeat, in seconds. */
export const leaseSeconds = {};

/** How often the open editor sends a heartbeat, in seconds. */
export const heartbeatSeconds = {};

/** How often the browser asks for the versions of what is on its screen, in seconds. */
export const watchSeconds = {};

/** How long a draft is kept after its last change, in days. */
export const draftKeepDays = {};

/** The most characters of the text of a draft. */
export const draftTextMax = {};

/** The most records one check of versions asks about. */
export const versionsMax = {};

/** The name of the part of a layout that its edit lock is about: a layout is edited as a whole. */
export const layoutEditPart = {layout_part:?};
",
        number("LEASE_SECONDS")?,
        number("HEARTBEAT_SECONDS")?,
        number("WATCH_SECONDS")?,
        number("DRAFT_KEEP_DAYS")?,
        number("DRAFT_TEXT_MAX")?,
        number("VERSIONS_MAX")?,
    ))
}

/// The text of `const NAME: &str = "…";` in a Rust source.
fn text_const(source: &str, name: &str) -> Option<String> {
    let file = syn::parse_file(source).ok()?;
    file.items.iter().find_map(|item| match item {
        syn::Item::Const(item) if item.ident == name => match &*item.expr {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(text),
                ..
            }) => Some(text.value()),
            _ => None,
        },
        _ => None,
    })
}

/// The scale of the decimal numbers of own fields as constants for the browser client: how many
/// parts a whole has, and how many digits that is after the point.
pub(super) fn fields(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == FIELDS_SOURCE) else {
        bail!("{FIELDS_SOURCE} is missing");
    };
    let file = syn::parse_file(text).with_context(|| format!("{FIELDS_SOURCE} is not Rust"))?;
    let parts = number_const(&file, FIELDS_SOURCE, "DECIMAL_PARTS")?;
    let digits = parts.len().saturating_sub(1);
    let prefix = text_const(text, "EDIT_PART_PREFIX")
        .with_context(|| format!("{FIELDS_SOURCE} has no `const EDIT_PART_PREFIX` that is text"))?;
    Ok(format!(
        "{TS_HEADER}/** What the name of the edit part of a long-text own field starts with, before the ID of the field. */
export const ownFieldPartPrefix = {prefix:?};

/** How many parts a whole has in a decimal number of an own field (it is stored in these parts). */
export const decimalParts = {parts};

/** How many digits a decimal number of an own field has after the point. */
export const decimalDigits = {digits};
"
    ))
}

/// The name of the Trusted Types policy as a constant for the browser client: the one definition
/// is the macro in `bb-api-words`, which the content security policy uses too.
pub(super) fn browser_names(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == WORDS_SOURCE) else {
        bail!("{WORDS_SOURCE} is missing");
    };
    let name = text
        .split_once(POLICY_MACRO)
        .and_then(|(_, rest)| rest.split('"').nth(1))
        .with_context(|| format!("{WORDS_SOURCE} has no `{POLICY_MACRO}` with a name"))?;
    Ok(format!(
        "{TS_HEADER}/** The one Trusted Types policy that may make HTML (the content security policy names it). */
export const trustedTypesPolicyName = \"{name}\";
"
    ))
}

#[cfg(test)]
mod tests {
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
"
            .to_owned(),
        );
        let text = editing(&[source]).unwrap();
        assert!(text.contains("leaseSeconds = 60;"), "{text}");
        assert!(text.contains("heartbeatSeconds = 15;"), "{text}");
        assert!(text.contains("watchSeconds = 30;"), "{text}");
        assert!(text.contains("draftKeepDays = 30;"), "{text}");
        assert!(text.contains("draftTextMax = 100000;"), "{text}");
        assert!(text.contains("versionsMax = 50;"), "{text}");
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
        let text = words(&spec).unwrap();
        assert!(
            text.contains("export const issueIconWords = [\"bug\", \"flag\"] as const;"),
            "{text}"
        );
        assert!(text.contains("export const choiceColorWords = [\"bug\", \"flag\"] as const;"));
        assert!(words("{}").is_err());
        assert!(words("not json").is_err());
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
            "pub const PAGE_SIZE_MAX: u32 = 100;\npub const PAGE_SIZE_DEFAULT: u32 = 50;\n"
                .to_owned(),
        );
        let text = paging(&[source]).unwrap();
        assert!(text.contains("pageSizeMax = 100;"), "{text}");
        assert!(text.contains("pageSizeDefault = 50;"), "{text}");
        assert!(paging(&[]).is_err());
    }
}
