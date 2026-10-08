//! What the browser app takes from the Rust code as constants, so it never writes them a second
//! time: the keys, the choices and the defaults of the settings (ADR-0018), and the page sizes of
//! the lists (PERF-02).
//!
//! xtask does not link the product crates, so both are read as Rust source.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::{Context as _, bail};

use super::TS_HEADER;
use crate::checks::arch_settings::{CATALOG_PATH, REGISTRY_FOLDER};
use crate::rust_source::Source;

const PAGING_SOURCE: &str = "crates/bb-core/src/paging.rs";
const PAGE_SIZE_MAX: &str = "PAGE_SIZE_MAX";
const PAGE_SIZE_DEFAULT: &str = "PAGE_SIZE_DEFAULT";

/// What one `settings! { … }` declaration says that the browser needs.
#[derive(Debug, PartialEq, Eq)]
struct SettingDecl {
    key: String,
    /// The default, already written the way TypeScript writes it.
    default: String,
    /// The name of the constant that lists the choices, for a setting of `Format::Choices`.
    choices: Option<String>,
}

/// One `name: expression` line inside the braces of a declaration.
struct Field {
    name: syn::Ident,
    value: syn::Expr,
}

impl syn::parse::Parse for Field {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        Ok(Self {
            name,
            value: input.parse()?,
        })
    }
}

/// The declarations of a `settings! { … }` body: `NAME: Type = { field: value, … }`, one after the
/// other.
struct Declarations(Vec<SettingDecl>);

impl syn::parse::Parse for Declarations {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut found = Vec::new();
        while !input.is_empty() {
            syn::Attribute::parse_outer(input)?;
            let name: syn::Ident = input.parse()?;
            input.parse::<syn::Token![:]>()?;
            input.parse::<syn::Ident>()?;
            input.parse::<syn::Token![=]>()?;
            let body;
            syn::braced!(body in input);
            let fields =
                syn::punctuated::Punctuated::<Field, syn::Token![,]>::parse_terminated(&body)?;
            found.push(declaration(&name, fields.iter())?);
        }
        Ok(Self(found))
    }
}

fn declaration<'a>(
    name: &syn::Ident,
    fields: impl Iterator<Item = &'a Field>,
) -> syn::Result<SettingDecl> {
    let mut key = None;
    let mut default = None;
    let mut choices = None;
    for field in fields {
        match field.name.to_string().as_str() {
            "key" => key = literal(&field.value).map(|(text, _)| text),
            "default" => default = literal(&field.value).map(|(_, typescript)| typescript),
            "format" => choices = choices_of(&field.value)?,
            _ => {}
        }
    }
    match (key, default) {
        (Some(key), Some(default)) => Ok(SettingDecl {
            key,
            default,
            choices,
        }),
        _ => Err(syn::Error::new(
            name.span(),
            format!("the setting {name} has no `key` or no `default` that is a plain value"),
        )),
    }
}

/// A literal as its text and as TypeScript writes it: `"device"`, `false`, `90`.
fn literal(expr: &syn::Expr) -> Option<(String, String)> {
    let syn::Expr::Lit(syn::ExprLit { lit, .. }) = expr else {
        return None;
    };
    match lit {
        syn::Lit::Str(text) => Some((text.value(), format!("{:?}", text.value()))),
        syn::Lit::Bool(flag) => Some((flag.value.to_string(), flag.value.to_string())),
        syn::Lit::Int(number) => Some((number.to_string(), number.base10_digits().to_owned())),
        _ => None,
    }
}

/// `Format::Choices(NAME)` as `NAME`; nothing for any other format. A format that says `Choices`
/// but not as `Choices(NAME)` is an error: its choices would be left out of the browser's list
/// without a word.
fn choices_of(expr: &syn::Expr) -> syn::Result<Option<String>> {
    let syn::Expr::Call(call) = expr else {
        return Ok(None);
    };
    let syn::Expr::Path(function) = &*call.func else {
        return Ok(None);
    };
    if function
        .path
        .segments
        .last()
        .is_none_or(|segment| segment.ident != "Choices")
    {
        return Ok(None);
    }
    match call.args.first() {
        Some(syn::Expr::Path(name)) if call.args.len() == 1 => Ok(name
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())),
        _ => Err(syn::Error::new(
            call.paren_token.span.open(),
            "`Choices(…)` must name one list of choices, like `Choices(MODE_CHOICES)`",
        )),
    }
}

/// The values of `const NAME: &[Choice] = &[Choice::new("value", label), …];`.
fn choice_values(item: &syn::ItemConst) -> Option<Vec<String>> {
    let syn::Expr::Reference(reference) = &*item.expr else {
        return None;
    };
    let syn::Expr::Array(array) = &*reference.expr else {
        return None;
    };
    array
        .elems
        .iter()
        .map(|element| {
            let syn::Expr::Call(call) = element else {
                return None;
            };
            literal(call.args.first()?).map(|(text, _)| text)
        })
        .collect()
}

/// The lists of choices by the name of the constant that holds them.
type ChoiceLists = BTreeMap<String, Vec<String>>;

/// The settings of the registry in the order of the file, and the lists of choices by name. The
/// settings that exist only for tests (`pub mod for_tests`) are left out.
fn read_registry(path: &str, text: &str) -> anyhow::Result<(Vec<SettingDecl>, ChoiceLists)> {
    let file = syn::parse_file(text).with_context(|| format!("{path} is not Rust"))?;
    let mut settings = Vec::new();
    let mut lists = BTreeMap::new();
    for item in &file.items {
        match item {
            syn::Item::Const(item) => {
                if let Some(values) = choice_values(item) {
                    lists.insert(item.ident.to_string(), values);
                }
            }
            syn::Item::Macro(item)
                if item
                    .mac
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "settings") =>
            {
                let declarations: Declarations = item
                    .mac
                    .parse_body()
                    .with_context(|| format!("{path}: a `settings! {{ … }}` body"))?;
                settings.extend(declarations.0);
            }
            _ => {}
        }
    }
    Ok((settings, lists))
}

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
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| path.starts_with(REGISTRY_FOLDER) && path.ends_with("/catalog.rs"))
    {
        seen = true;
        let (found, choices) = read_registry(path, text)?;
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
