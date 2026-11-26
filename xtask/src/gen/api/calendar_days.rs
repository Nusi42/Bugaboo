//! The most days one answer of the calendar covers, for the browser client (PLAN-09).

use anyhow::{Context, bail};

use super::web_constants::number_const;
use super::{CALENDAR_PATH, Generated, TS_HEADER, format_with_oxfmt};
use crate::repo::Repo;
use crate::source::Source;

const CALENDAR_SOURCE: &str = "crates/bb-app/src/calendar/types.rs";

/// The most days one answer of the calendar covers, as a constant for the browser client: a phone
/// lists as many days from today.
pub(super) fn calendar_days(sources: &[Source]) -> anyhow::Result<String> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == CALENDAR_SOURCE) else {
        bail!("{CALENDAR_SOURCE} is missing");
    };
    let file = syn::parse_file(text).with_context(|| format!("{CALENDAR_SOURCE} is not Rust"))?;
    Ok(format!(
        "{TS_HEADER}/** The most days one answer of the calendar covers. */
export const calendarDaysMax = {};
",
        number_const(&file, CALENDAR_SOURCE, "CALENDAR_DAYS_MAX")?,
    ))
}

/// The generated file of the constant, formatted as the others are.
pub(super) fn generated(repo: &Repo, sources: &[Source]) -> anyhow::Result<Generated> {
    Ok(Generated::new(
        CALENDAR_PATH,
        format_with_oxfmt(repo, CALENDAR_PATH, &calendar_days(sources)?)?,
    ))
}
