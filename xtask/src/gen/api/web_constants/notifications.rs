//! What the screens of notifications, the mail server, signing up and handles take from the Rust
//! code, so their texts and tables never write a number or a key a second time:
//!
//! - the table of *Customize*: for each kind of notification, the setting that says whether the
//!   person hears of it in the app and the one for e-mail (`NotificationKind::setting`), or `null`
//!   for a kind nobody can switch off;
//! - the numbers the texts say: how long notifications are kept, how long the link of a sign-up
//!   works and how long a new one waits, how often a test e-mail may go, how often a handle may be
//!   changed and how long it may be.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::{Context as _, bail};

use super::{TS_HEADER, number_const, registry};
use crate::source::Source;

pub(super) const NOTIFICATION_SOURCE: &str = "crates/bb-core/src/notification.rs";
pub(super) const MAIL_SOURCE: &str = "crates/bb-core/src/mail.rs";
pub(super) const SIGN_UP_SOURCE: &str = "crates/bb-core/src/sign_up_invitation.rs";
pub(super) const HANDLE_SOURCE: &str = "crates/bb-core/src/handle.rs";
const COMMENT_SOURCE: &str = "crates/bb-core/src/comment.rs";

/// The name of the type the table is declared on, and of the function that holds it.
const KIND_TYPE: &str = "NotificationKind";
const SETTING_FUNCTION: &str = "setting";

/// The two ways a person can hear of something, by the name of the variant of `Via`.
const APP: &str = "App";
const MAIL: &str = "Mail";

pub(super) fn parsed(sources: &[Source], path: &str) -> anyhow::Result<syn::File> {
    let Some((_, text)) = sources.iter().find(|(known, _)| known == path) else {
        bail!("{path} is missing");
    };
    syn::parse_file(text).with_context(|| format!("{path} is not Rust"))
}

/// `followed-activity` as `FollowedActivity`.
fn pascal_case(word: &str) -> String {
    word.split('-')
        .filter_map(|part| {
            let mut letters = part.chars();
            letters
                .next()
                .map(|first| first.to_uppercase().chain(letters).collect::<String>())
        })
        .collect()
}

/// The last name of a path pattern or expression: `Self::Mentioned` is `Mentioned`.
fn last_name(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
}

/// The name a pattern like `Self::Mentioned` stands for.
fn pattern_name(pattern: &syn::Pat) -> Option<String> {
    match pattern {
        syn::Pat::Path(path) => last_name(&path.path),
        _ => None,
    }
}

/// The setting an arm of the match names: `Some(&CONSTANT)` (in braces or not), or `None` for a
/// kind nobody can switch off.
fn constant_of(body: &syn::Expr) -> anyhow::Result<Option<String>> {
    let not_read =
        || anyhow::anyhow!("an arm of `{KIND_TYPE}::{SETTING_FUNCTION}` does not name a setting");
    match body {
        syn::Expr::Path(path) if last_name(&path.path).as_deref() == Some("None") => Ok(None),
        syn::Expr::Call(call) => {
            let (syn::Expr::Path(function), Some(syn::Expr::Reference(reference))) =
                (&*call.func, call.args.first())
            else {
                return Err(not_read());
            };
            let syn::Expr::Path(constant) = &*reference.expr else {
                return Err(not_read());
            };
            match (
                last_name(&function.path).as_deref(),
                last_name(&constant.path),
            ) {
                (Some("Some"), Some(constant)) => Ok(Some(constant)),
                _ => Err(not_read()),
            }
        }
        // A long arm is written in braces by the formatter: the one expression inside counts.
        syn::Expr::Block(block) => match block.block.stmts.as_slice() {
            [syn::Stmt::Expr(inner, None)] => constant_of(inner),
            _ => Err(not_read()),
        },
        _ => Err(not_read()),
    }
}

/// The match of `NotificationKind::setting`, as `(variant, way, constant)` for each arm that names
/// a setting. An arm that names none is a kind nobody can switch off, and is left out.
fn arms(file: &syn::File) -> anyhow::Result<Vec<(String, String, String)>> {
    let function = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item) => Some(item),
            _ => None,
        })
        .filter(|item| {
            matches!(&*item.self_ty, syn::Type::Path(path) if last_name(&path.path).as_deref() == Some(KIND_TYPE))
        })
        .flat_map(|item| &item.items)
        .find_map(|item| match item {
            syn::ImplItem::Fn(function) if function.sig.ident == SETTING_FUNCTION => Some(function),
            _ => None,
        })
        .with_context(|| format!("{NOTIFICATION_SOURCE} has no `{KIND_TYPE}::{SETTING_FUNCTION}`"))?;
    let table = function
        .block
        .stmts
        .iter()
        .find_map(|statement| match statement {
            syn::Stmt::Expr(syn::Expr::Match(table), _) => Some(table),
            _ => None,
        })
        .with_context(|| format!("`{KIND_TYPE}::{SETTING_FUNCTION}` is not a `match`"))?;
    let mut found = Vec::new();
    for arm in &table.arms {
        let syn::Pat::Tuple(pair) = &arm.pat else {
            bail!("an arm of `{KIND_TYPE}::{SETTING_FUNCTION}` is not a pair");
        };
        let Some(constant) = constant_of(&arm.body)? else {
            continue;
        };
        let (kind, way) = (
            pair.elems.first().and_then(pattern_name),
            pair.elems.get(1).and_then(pattern_name),
        );
        match (kind, way) {
            (Some(kind), Some(way)) => found.push((kind, way, constant)),
            _ => bail!("an arm of `{KIND_TYPE}::{SETTING_FUNCTION}` is not read"),
        }
    }
    Ok(found)
}

/// The words of `NotificationKind` in the order the server lists them.
fn kind_words(spec: &str) -> anyhow::Result<Vec<String>> {
    let spec: serde_json::Value =
        serde_json::from_str(spec).context("the API description is not JSON")?;
    spec["components"]["schemas"][KIND_TYPE]["enum"]
        .as_array()
        .with_context(|| format!("the API description has no list of words `{KIND_TYPE}`"))?
        .iter()
        .map(|word| {
            word.as_str()
                .map(str::to_owned)
                .with_context(|| format!("`{KIND_TYPE}` has a word that is not text"))
        })
        .collect()
}

/// The table of *Customize*, as TypeScript: for each kind (by its word), the keys of its two
/// settings.
fn settings_table(sources: &[Source], spec: &str) -> anyhow::Result<String> {
    let arms = arms(&parsed(sources, NOTIFICATION_SOURCE)?)?;
    let (settings, _) = registry(sources)?;
    let key_of = |constant: &str| {
        settings
            .iter()
            .find(|setting| setting.ident == constant)
            .map(|setting| setting.key.clone())
            .with_context(|| format!("the registry has no setting `{constant}`"))
    };
    let mut table = String::new();
    for word in kind_words(spec)? {
        let variant = pascal_case(&word);
        let mut keys = BTreeMap::new();
        for (kind, way, constant) in &arms {
            if *kind == variant {
                keys.insert(way.as_str(), key_of(constant)?);
            }
        }
        // A word with a hyphen is not a name in TypeScript, so it is written as a string.
        let key = if word.contains('-') {
            format!("{word:?}")
        } else {
            word.clone()
        };
        // A kind with no setting at all is one nobody can switch off: it has no switches.
        if keys.is_empty() {
            let _ = writeln!(table, "  {key}: null,");
            continue;
        }
        let (Some(app), Some(mail)) = (keys.get(APP), keys.get(MAIL)) else {
            bail!("`{KIND_TYPE}::{SETTING_FUNCTION}` has no setting for both ways of `{word}`");
        };
        let _ = writeln!(table, "  {key}: {{ app: {app:?}, mail: {mail:?} }},");
    }
    Ok(table)
}

/// The numbers the texts say and the table of *Customize*, as constants for the browser client.
pub(crate) fn notifications(sources: &[Source], spec: &str) -> anyhow::Result<String> {
    let number = |path: &str, name: &str| number_const(&parsed(sources, path)?, path, name);
    Ok(format!(
        "{TS_HEADER}/** For each kind of notification, the settings that say whether a person hears of it in the app and by e-mail (*Customize*); `null` for a kind nobody can switch off. */
export const notificationSettings = {{
{table}}} as const;

/** The most unread notifications the server counts: a count of this many means this many or more. */
export const notificationsAtOnceMax = {at_once};

/** How many hours the link that signing up sends works. */
export const signUpLinkHours = {link_hours};

/** How many minutes a new link for an address waits for the one before it. */
export const signUpResendMinutes = {resend};

/** How many seconds a person waits between two test e-mails. */
export const testEmailEverySeconds = {test_every};

/** How many days a person waits between two changes of their handle. */
export const handleChangeEveryDays = {handle_days};

/** The most characters a handle has. */
export const handleMax = {handle_max};
",
        table = settings_table(sources, spec)?,
        at_once = number(NOTIFICATION_SOURCE, "AT_ONCE_MAX")?,
        link_hours = number(SIGN_UP_SOURCE, "HOURS")?,
        resend = number(SIGN_UP_SOURCE, "RESEND_AFTER_MINUTES")?,
        test_every = number(MAIL_SOURCE, "TEST_EMAIL_EVERY_SECONDS")?,
        handle_days = number(HANDLE_SOURCE, "CHANGE_EVERY_DAYS")?,
        handle_max = number(COMMENT_SOURCE, "HANDLE_MAX")?,
    ))
}

#[cfg(test)]
mod tests;
