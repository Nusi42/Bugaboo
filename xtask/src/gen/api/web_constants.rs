//! What the browser app takes from the Rust code as constants, so it never writes them a second
//! time: the keys, the choices and the defaults of the settings (ADR-0018), and the page sizes of
//! the lists (PERF-02).
//!
//! xtask does not link the product crates, so both are read as Rust source.

use std::fmt::Write as _;

use anyhow::{Context as _, bail};

use super::TS_HEADER;
use crate::checks::arch_settings::{CATALOG_PATH, REGISTRY_FOLDER};
use crate::rust_registry::{ChoiceLists, read_registry};
use crate::source::Source;

const PAGING_SOURCE: &str = "crates/bb-core/src/paging.rs";
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

/// The value of `pub const NAME: u32 = 100;` in `text`.
fn number_const(file: &syn::File, name: &str) -> anyhow::Result<String> {
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
        .with_context(|| format!("{PAGING_SOURCE} has no `const {name}` that is a number"))
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
        number_const(&file, PAGE_SIZE_MAX)?,
        number_const(&file, PAGE_SIZE_DEFAULT)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
