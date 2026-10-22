//! Reading the settings registry (`crates/bb-core/src/settings/catalog.rs`) as Rust: the one
//! reader of the `settings! { … }` declarations and of the lists of choices they name. ARCH-SETTINGS
//! (which keys exist) and the generator of the browser constants (keys, choices, defaults) both ask
//! here, so the registry has one parser (coding-standards §1).
//!
//! The lists of choices and the constants that a default names may stand in another file of the
//! registry folder: the theme generator writes the themes there (ADR-0026).

use std::collections::BTreeMap;

use anyhow::Context as _;

/// The default of a declaration: written out, or the name of a constant.
#[derive(Debug, PartialEq, Eq)]
enum Default {
    /// Already written the way TypeScript writes it.
    Literal(String),
    Constant(String),
}

/// What one `settings! { … }` declaration says that the tools need.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SettingDecl {
    /// The name of the constant the setting is declared as, like `MAIL_HOST`.
    pub(crate) ident: String,
    pub(crate) key: String,
    /// The default, already written the way TypeScript writes it.
    pub(crate) default: String,
    /// The name of the constant that lists the choices, for a setting of `Format::Choices`.
    pub(crate) choices: Option<String>,
}

/// A declaration before its default is looked up.
struct Declared {
    ident: String,
    key: String,
    default: Default,
    choices: Option<String>,
}

/// The lists of choices by the name of the constant that holds them.
pub(crate) type ChoiceLists = BTreeMap<String, Vec<String>>;

/// The lists of choices and the constants with a plain value (as TypeScript writes the value) that
/// the files of the registry hold.
#[derive(Default)]
struct Shared {
    lists: ChoiceLists,
    constants: BTreeMap<String, String>,
}

impl Shared {
    fn add(&mut self, item: &syn::ItemConst) {
        if let Some(values) = choice_values(item, &self.constants) {
            self.lists.insert(item.ident.to_string(), values);
        } else if let Some((_, typescript)) = literal(&item.expr) {
            self.constants.insert(item.ident.to_string(), typescript);
        }
    }

    /// The constants of a file that declares no settings itself (the generated theme choices).
    fn add_file(&mut self, text: &str) -> anyhow::Result<()> {
        let file = syn::parse_file(text).context("a file of the registry is not Rust")?;
        for item in &file.items {
            if let syn::Item::Const(item) = item {
                self.add(item);
            }
        }
        Ok(())
    }
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
struct Declarations(Vec<Declared>);

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
) -> syn::Result<Declared> {
    let mut key = None;
    let mut default = None;
    let mut choices = None;
    for field in fields {
        match field.name.to_string().as_str() {
            "key" => key = literal(&field.value).map(|(text, _)| text),
            "default" => default = default_of(&field.value),
            "format" => choices = choices_of(&field.value)?,
            _ => {}
        }
    }
    match (key, default) {
        (Some(key), Some(default)) => Ok(Declared {
            ident: name.to_string(),
            key,
            default,
            choices,
        }),
        _ => Err(syn::Error::new(
            name.span(),
            format!(
                "the setting {name} has no `key` or no `default` that is a plain value or the name of a constant"
            ),
        )),
    }
}

/// A default that is a literal, or the name of a constant that holds one.
fn default_of(expr: &syn::Expr) -> Option<Default> {
    if let Some((_, typescript)) = literal(expr) {
        return Some(Default::Literal(typescript));
    }
    let syn::Expr::Path(path) = expr else {
        return None;
    };
    path.path
        .get_ident()
        .map(|name| Default::Constant(name.to_string()))
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

/// The values of `const NAME: &[Choice] = &[Choice::new("value", label), …];`. A value may be the
/// name of a constant that holds the text (declared earlier), so the code that needs the value
/// uses the same constant.
fn choice_values(
    item: &syn::ItemConst,
    constants: &BTreeMap<String, String>,
) -> Option<Vec<String>> {
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
            let value = call.args.first()?;
            if let Some((text, _)) = literal(value) {
                return Some(text);
            }
            let syn::Expr::Path(path) = value else {
                return None;
            };
            let typescript = constants.get(&path.path.get_ident()?.to_string())?;
            serde_json::from_str(typescript).ok()
        })
        .collect()
}

/// Whether the macro call is `settings! { … }`.
fn is_settings(item: &syn::ItemMacro) -> bool {
    item.mac
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "settings")
}

/// Every declaration of the items, with whether it stands in a module (the settings that exist
/// only for tests do), and the lists of choices and constants.
fn walk(
    items: &[syn::Item],
    nested: bool,
    found: &mut Vec<(Declared, bool)>,
    shared: &mut Shared,
) -> anyhow::Result<()> {
    for item in items {
        match item {
            syn::Item::Const(item) => shared.add(item),
            syn::Item::Macro(item) if is_settings(item) => {
                let declarations: Declarations =
                    item.mac.parse_body().context("a `settings! { … }` body")?;
                found.extend(declarations.0.into_iter().map(|decl| (decl, nested)));
            }
            syn::Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    walk(inner, true, found, shared)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn read_all(text: &str) -> anyhow::Result<(Vec<(Declared, bool)>, Shared)> {
    let file = syn::parse_file(text).context("the registry is not Rust")?;
    let mut found = Vec::new();
    let mut shared = Shared::default();
    walk(&file.items, false, &mut found, &mut shared)?;
    Ok((found, shared))
}

/// The settings of the registry in the order of the file, and the lists of choices by name. The
/// settings that exist only for tests (`pub mod for_tests`) are left out. `others` are the other
/// files of the registry folder: the lists of choices and the constants they hold (like the
/// starting theme) count as if they stood in the catalog.
///
/// # Errors
/// A text is not Rust, a declaration has no `key` or no plain `default`, a default names a
/// constant that is nowhere, or a format of choices does not name its list.
pub(crate) fn read_registry(
    text: &str,
    others: &[&str],
) -> anyhow::Result<(Vec<SettingDecl>, ChoiceLists)> {
    let (found, mut shared) = read_all(text)?;
    for other in others {
        shared.add_file(other)?;
    }
    let mut settings = Vec::new();
    for (decl, _) in found.into_iter().filter(|(_, nested)| !nested) {
        let default = match decl.default {
            Default::Literal(typescript) => typescript,
            Default::Constant(name) => shared.constants.get(&name).cloned().with_context(|| {
                format!(
                    "the default of `{}` names the constant `{name}`, which has no plain value in the registry folder",
                    decl.key
                )
            })?,
        };
        settings.push(SettingDecl {
            ident: decl.ident,
            key: decl.key,
            default,
            choices: decl.choices,
        });
    }
    Ok((settings, shared.lists))
}

/// The keys of every setting in the text, also those that exist only for tests.
///
/// # Errors
/// The text is not Rust, or a declaration has no `key` or no plain `default`.
pub(crate) fn declared_keys(text: &str) -> anyhow::Result<Vec<String>> {
    Ok(read_all(text)?
        .0
        .into_iter()
        .map(|(decl, _)| decl.key)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: &str = r#"
const MODE_CHOICES: &[Choice] = &[Choice::new("device", a), Choice::new("light", b)];
settings! {
    /// The mode.
    MODE: Text = {
        key: "appearance.mode",
        default: "device",
        format: Format::Choices(MODE_CHOICES),
    }
    SIZE: Number = {
        key: "web.size",
        default: 90,
    }
}
pub mod for_tests {
    settings! {
        ONLY_IN_TESTS: Bool = {
            key: "test.only",
            default: false,
        }
    }
}
"#;

    /// A setting whose choices and default stand in another file, like the generated themes.
    const SPLIT: &str = r#"
settings! {
    LOOK: Text = {
        key: "appearance.look",
        default: DEFAULT_LOOK,
        format: Format::Choices(LOOK_CHOICES),
    }
}
"#;

    const OTHER: &str = r#"
pub(super) const LOOK_CHOICES: &[Choice] = &[Choice::new("day", a), Choice::new("night", b)];
pub(super) const DEFAULT_LOOK: &str = "day";
"#;

    #[test]
    fn good_example_keys_defaults_and_choices_are_read() {
        let (settings, lists) = read_registry(REGISTRY, &[]).unwrap();
        let keys: Vec<&str> = settings.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["appearance.mode", "web.size"]);
        assert_eq!(settings[0].ident, "MODE");
        assert_eq!(settings[0].default, "\"device\"");
        assert_eq!(settings[0].choices.as_deref(), Some("MODE_CHOICES"));
        assert_eq!(settings[1].default, "90");
        assert_eq!(lists["MODE_CHOICES"], ["device", "light"]);
    }

    #[test]
    fn good_example_a_choice_may_name_a_constant_that_holds_its_value() {
        let named = r#"
pub const DAY: &str = "day";
const LOOK_CHOICES: &[Choice] = &[Choice::new(DAY, a), Choice::new("night", b)];
"#;
        let (_, shared) = read_all(named).unwrap();
        assert_eq!(shared.lists["LOOK_CHOICES"], ["day", "night"]);
    }

    #[test]
    fn good_example_choices_and_a_default_from_another_file_are_read() {
        let (settings, lists) = read_registry(SPLIT, &[OTHER]).unwrap();
        assert_eq!(settings[0].default, "\"day\"");
        assert_eq!(settings[0].choices.as_deref(), Some("LOOK_CHOICES"));
        assert_eq!(lists["LOOK_CHOICES"], ["day", "night"]);
    }

    #[test]
    fn bad_example_a_default_that_names_no_constant_fails() {
        assert!(read_registry(SPLIT, &[]).is_err());
    }

    #[test]
    fn the_settings_for_tests_are_declared_keys_but_not_in_the_registry() {
        assert_eq!(
            declared_keys(REGISTRY).unwrap(),
            ["appearance.mode", "web.size", "test.only"]
        );
    }

    #[test]
    fn bad_example_a_declaration_without_a_plain_default_or_a_text_that_is_no_rust_fails() {
        let no_default = "settings! { A: Text = { key: \"a.b\", } }";
        assert!(read_registry(no_default, &[]).is_err());
        assert!(declared_keys(no_default).is_err());
        assert!(read_registry("settings! {", &[]).is_err());
        assert!(read_registry("fn nothing() {}", &[]).unwrap().0.is_empty());
    }

    #[test]
    fn bad_example_a_format_of_choices_that_names_no_single_list_fails() {
        let broken = REGISTRY.replace("Format::Choices(MODE_CHOICES)", "Format::Choices(A, B)");
        assert!(read_registry(&broken, &[]).is_err());
        assert!(declared_keys(&broken).is_err());
    }
}
