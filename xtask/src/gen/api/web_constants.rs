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
use crate::rust_registry::{ChoiceLists, SettingDecl, read_registry};
use crate::source::Source;

const APP_PATHS_SOURCE: &str = "crates/bb-core/src/app_paths.rs";
const WORDS_SOURCE: &str = "crates/bb-api-words/src/lib.rs";
const POLICY_MACRO: &str = "macro_rules! trusted_types_policy";
const PAGING_SOURCE: &str = "crates/bb-core/src/paging.rs";
const EDIT_SOURCE: &str = "crates/bb-core/src/edit.rs";
const COMMENT_SOURCE: &str = "crates/bb-core/src/comment.rs";
const ISSUE_SOURCE: &str = "crates/bb-core/src/issue.rs";
const ERROR_CODE_SOURCE: &str = "crates/bb-core/src/error_code.rs";
const VERSION_SOURCE: &str = "crates/bb-core/src/version.rs";
const TIME_SOURCE: &str = "crates/bb-core/src/time.rs";
const DRAFT_KEPT_LIST: &str = "DRAFT_KEPT_BY";
const FIELDS_SOURCE: &str = "crates/bb-core/src/own_fields.rs";
const FILES_SOURCE: &str = "crates/bb-core/src/files.rs";
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

/// The settings of the registry in the order of the file, and the lists of choices by name. The
/// settings that exist only for tests are left out.
pub(super) fn registry(sources: &[Source]) -> anyhow::Result<(Vec<SettingDecl>, ChoiceLists)> {
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
    Ok((settings, lists))
}

/// The keys, the choices and the defaults of the registry as constants for the browser client. The
/// settings that exist only for tests are left out.
pub(super) fn setting_keys(sources: &[Source]) -> anyhow::Result<String> {
    let (settings, lists) = registry(sources)?;
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
            syn::Item::Const(item) if item.ident == name => {
                number_of(&item.expr).map(|number| number.to_string())
            }
            _ => None,
        })
        .with_context(|| format!("{path} has no `const {name}` that is a number"))
}

/// The value of a whole number written as a literal or as a product of literals (`4 * 1024 * 1024`).
fn number_of(expr: &syn::Expr) -> Option<u128> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(number),
            ..
        }) => number.base10_parse().ok(),
        syn::Expr::Binary(syn::ExprBinary {
            left,
            op: syn::BinOp::Mul(_),
            right,
            ..
        }) => number_of(left)?.checked_mul(number_of(right)?),
        syn::Expr::Paren(inner) => number_of(&inner.expr),
        _ => None,
    }
}

/// The vocabularies the browser lists or recognizes at run time (a drop-down of icons, the words of
/// a priority that a value must be one of), by the name of their
/// schema in the API description. The type of each stays the generated one; only the list of words
/// is made here, so it is never written a second time.
const WORD_LISTS: [&str; 13] = [
    "IssueIcon",
    "StatusGroup",
    "Priority",
    "Field",
    "FieldType",
    "FieldFor",
    "Look",
    "ChoiceColor",
    "LayoutKind",
    "HistoryTab",
    "NotificationKind",
    "NotificationTab",
    "OnlyIfTest",
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
pub(super) fn words(spec: &str, sources: &[Source]) -> anyhow::Result<String> {
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
    let Some((_, issues)) = sources.iter().find(|(path, _)| path == ISSUE_SOURCE) else {
        bail!("{ISSUE_SOURCE} is missing");
    };
    let _ = write!(
        out,
        "/** The priority a new issue has when the form does not say: the server decides, the form shows it. */
export const newIssuePriority = {:?};
",
        new_word(issues, "Priority")?
    );
    Ok(out)
}

/// The word of the variant that `impl Name { const NEW: Self = Self::Variant; }` names, from the
/// `words!` table of the enum `Name`.
fn new_word(source: &str, name: &str) -> anyhow::Result<String> {
    let file = syn::parse_file(source).with_context(|| format!("{ISSUE_SOURCE} is not Rust"))?;
    let variant = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Impl(item) if is_type(&item.self_ty, name) => {
                item.items.iter().find_map(|each| match each {
                    syn::ImplItem::Const(each) if each.ident == "NEW" => match &each.expr {
                        syn::Expr::Path(path) => {
                            path.path.segments.last().map(|s| s.ident.to_string())
                        }
                        _ => None,
                    },
                    _ => None,
                })
            }
            _ => None,
        })
        .with_context(|| {
            format!("{ISSUE_SOURCE} has no `impl {name} {{ const NEW: Self = Self::… }}`")
        })?;
    let table = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Macro(item) if item.mac.path.is_ident("words") => {
                let text = item.mac.tokens.to_string();
                text.contains(&format!("enum {name} ")).then_some(text)
            }
            _ => None,
        })
        .with_context(|| format!("{ISSUE_SOURCE} has no `words!` table for `{name}`"))?;
    let after = table
        .split_once(&format!("{variant} =>"))
        .and_then(|(_, rest)| rest.split('"').nth(1))
        .with_context(|| format!("`{name}` has no word for `{variant}`"))?;
    Ok(after.to_owned())
}

/// Whether the type is the plain name `name`.
fn is_type(ty: &syn::Type, name: &str) -> bool {
    matches!(ty, syn::Type::Path(path) if path.path.is_ident(name))
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
    let kept = draft_kept_codes(&file, sources)?
        .iter()
        .map(|code| format!("{code:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let part = |name: &str| {
        text_const(text, name)
            .with_context(|| format!("{EDIT_SOURCE} has no `const {name}` that is text"))
    };
    let (layout_part, workflow_part, description_part) = (
        part("LAYOUT_PART")?,
        part("WORKFLOW_PART")?,
        part("DESCRIPTION_PART")?,
    );
    let Some((_, versions)) = sources.iter().find(|(path, _)| path == VERSION_SOURCE) else {
        bail!("{VERSION_SOURCE} is missing");
    };
    let versions =
        syn::parse_file(versions).with_context(|| format!("{VERSION_SOURCE} is not Rust"))?;
    let first_version = number_const(&versions, VERSION_SOURCE, "FIRST_VERSION")?;
    let Some((_, time)) = sources.iter().find(|(path, _)| path == TIME_SOURCE) else {
        bail!("{TIME_SOURCE} is missing");
    };
    let time = syn::parse_file(time).with_context(|| format!("{TIME_SOURCE} is not Rust"))?;
    let seconds_per_day = number_const(&time, TIME_SOURCE, "SECONDS_PER_DAY")?;
    let seconds_per_minute = number_const(&time, TIME_SOURCE, "SECONDS_PER_MINUTE")?;
    let minutes_per_hour = number_const(&time, TIME_SOURCE, "MINUTES_PER_HOUR")?;
    let Some((_, comments)) = sources.iter().find(|(path, _)| path == COMMENT_SOURCE) else {
        bail!("{COMMENT_SOURCE} is missing");
    };
    let comment_part = text_const(comments, "TEXT_PART")
        .with_context(|| format!("{COMMENT_SOURCE} has no `const TEXT_PART` that is text"))?;
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

/** The codes of the refusals of a save that keep the text as the person's draft. */
export const draftKeptCodes = [{}] as const;

/** One code of `draftKeptCodes`. */
export type DraftKeptCode = (typeof draftKeptCodes)[number];

/** The name of the part of a layout that its edit lock is about: a layout is edited as a whole. */
export const layoutEditPart = {layout_part:?};

/** The name of the part of a workflow that its edit lock is about: a workflow is edited as a whole and made to count with *Publish*. */
export const workflowEditPart = {workflow_part:?};

/** The name of the part of an issue that holds its description. */
export const descriptionEditPart = {description_part:?};

/** The name of the part of a comment that holds its text. */
export const commentEditPart = {comment_part:?};

/** The version a new record starts with. */
export const firstVersion = {first_version};

/** The seconds in a day: the keep periods and lifetimes counted in days are 24 hours long. */
export const secondsPerDay = {seconds_per_day};

/** The seconds in a minute. */
export const secondsPerMinute = {seconds_per_minute};

/** The minutes in an hour. */
export const minutesPerHour = {minutes_per_hour};
",
        number("LEASE_SECONDS")?,
        number("HEARTBEAT_SECONDS")?,
        number("WATCH_SECONDS")?,
        number("DRAFT_KEEP_DAYS")?,
        number("DRAFT_TEXT_MAX")?,
        number("VERSIONS_MAX")?,
        kept,
    ))
}

/// One row of the table of error codes: `Variant => ("text", status)`.
struct CodeRow {
    variant: String,
    code: String,
}

impl syn::parse::Parse for CodeRow {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        syn::Attribute::parse_outer(input)?;
        let variant: syn::Ident = input.parse()?;
        input.parse::<syn::Token![=>]>()?;
        let inner;
        syn::parenthesized!(inner in input);
        let code: syn::LitStr = inner.parse()?;
        inner.parse::<syn::Token![,]>()?;
        inner.parse::<syn::LitInt>()?;
        Ok(Self {
            variant: variant.to_string(),
            code: code.value(),
        })
    }
}

/// The text of every error code by its variant, from `error_codes! { … }`.
fn error_codes(sources: &[Source]) -> anyhow::Result<Vec<(String, String)>> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == ERROR_CODE_SOURCE) else {
        bail!("{ERROR_CODE_SOURCE} is missing");
    };
    let file = syn::parse_file(text).with_context(|| format!("{ERROR_CODE_SOURCE} is not Rust"))?;
    for item in &file.items {
        let syn::Item::Macro(item) = item else {
            continue;
        };
        if item.mac.path.is_ident("error_codes") {
            let rows = item
                .mac
                .parse_body_with(
                    syn::punctuated::Punctuated::<CodeRow, syn::Token![,]>::parse_terminated,
                )
                .with_context(|| {
                    format!("{ERROR_CODE_SOURCE}: a line of `error_codes!` is not `Name => (\"text\", status)`")
                })?;
            return Ok(rows
                .into_iter()
                .map(|row| (row.variant, row.code))
                .collect());
        }
    }
    bail!("{ERROR_CODE_SOURCE} has no `error_codes! {{ … }}` table")
}

/// The codes of the list `DRAFT_KEPT_BY` of the edit module (`[ErrorCode::A, ErrorCode::B]`), in
/// the order of the list.
fn draft_kept_codes(file: &syn::File, sources: &[Source]) -> anyhow::Result<Vec<String>> {
    let list = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Const(item) if item.ident == DRAFT_KEPT_LIST => match &*item.expr {
                syn::Expr::Array(array) => Some(array),
                _ => None,
            },
            _ => None,
        })
        .with_context(|| {
            format!("{EDIT_SOURCE} has no `const {DRAFT_KEPT_LIST}` that is a list")
        })?;
    let table = error_codes(sources)?;
    list.elems
        .iter()
        .map(|element| {
            let variant = match element {
                syn::Expr::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()),
                _ => None,
            }
            .with_context(|| format!("{EDIT_SOURCE}: `{DRAFT_KEPT_LIST}` holds more than `ErrorCode::Name`"))?;
            table
                .iter()
                .find(|(each, _)| *each == variant)
                .map(|(_, code)| code.clone())
                .with_context(|| format!("{DRAFT_KEPT_LIST} names `{variant}`, which {ERROR_CODE_SOURCE} does not list"))
        })
        .collect()
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

/// The sizes of files that the server and the browser both name: how big one part of an upload is
/// and what a megabyte is in the limits of files, so the tests of the browser do not write them a
/// second time.
pub(super) fn files(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == FILES_SOURCE) else {
        bail!("{FILES_SOURCE} is missing");
    };
    let file = syn::parse_file(text).with_context(|| format!("{FILES_SOURCE} is not Rust"))?;
    let part = number_const(&file, FILES_SOURCE, "PART_BYTES")?;
    let megabyte = number_const(&file, FILES_SOURCE, "MEGABYTE")?;
    Ok(format!(
        "{TS_HEADER}/** The most bytes one part of an upload holds (`PART_BYTES` of the server). */
export const partBytes = {part};

/** The size of one megabyte in the limits of files: the limit of a project is a whole number of them. */
export const megabyte = {megabyte};
"
    ))
}

/// The addresses of the app pages that the server links to as constants for the browser app, so
/// `paths.ts` does not write them a second time.
pub(super) fn app_paths(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == APP_PATHS_SOURCE) else {
        bail!("{APP_PATHS_SOURCE} is missing");
    };
    let path = |name: &str| {
        text_const(text, name)
            .with_context(|| format!("{APP_PATHS_SOURCE} has no `const {name}` that is text"))
    };
    let (issue, project_settings, invitation, sign_in, notifications) = (
        path("ISSUE")?,
        path("PROJECT_SETTINGS")?,
        path("INVITATION")?,
        path("SIGN_IN")?,
        path("NOTIFICATIONS")?,
    );
    Ok(format!(
        "{TS_HEADER}/** The addresses of the app pages that the server links to (invitations, e-mails). */
export const appPaths = {{
  issue: {issue:?},
  projectSettings: {project_settings:?},
  invitation: {invitation:?},
  signIn: {sign_in:?},
  notifications: {notifications:?},
}} as const;
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

mod notifications;
pub(super) use notifications::notifications;

#[cfg(test)]
mod tests;
