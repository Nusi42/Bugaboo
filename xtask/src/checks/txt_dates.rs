//! TXT-DATES: a date, a moment, a time of day and a length of time on a screen are written only by
//! the shared helpers (UX-07, CFG-04, coding-standards §4).
//!
//! The person's *Region* says how each is written, and one helper writes each kind
//! (`web/app/src/shared/format.ts` and its neighbours; the packages of `web/packages/`). A screen
//! that asks `Intl` itself, or a date for its `toLocale…String`, writes it a second way, which may
//! not follow the person's choices (a day first beside a clock of 12 hours, M5-F137). A finding is
//! one of [`WRITERS`] anywhere in `web/` outside the shared helpers ([`HELPERS`]), tests and tools
//! excluded (a test may work out what it expects another way).
//!
//! The browser's own box for a day or a time (`<input type="date">` and its kin) writes a day too, in
//! the format of the device, not the person's: a *Start date* read `23.11.2026` beside a *Created*
//! of `2026-11-13, 13:21` in the person's own format (M8 exit). Every box for a day is a text box
//! read and written by the shared helpers, so one of [`BROWSER_BOXES`] is a finding anywhere in
//! `web/`, the helpers and the packages included.

use super::web_source::{code_of, is_app_source};
use super::{Check, Context, Finding};
use crate::source::{Kind, Source};

/// The code of the browser.
const WEB: &str = "web/";
/// The shared helpers, which may ask `Intl` themselves: those of the app, and the packages.
const HELPERS: [&str; 2] = ["web/app/src/shared/", "web/packages/"];

/// What writes a date, a moment, a time or a length of time by itself.
const WRITERS: [&str; 5] = [
    "Intl.DateTimeFormat",
    "Intl.RelativeTimeFormat",
    "toLocaleDateString",
    "toLocaleTimeString",
    "toLocaleString",
];

/// The browser's own boxes for a day or a time, as code asks for them.
const BROWSER_BOXES: [&str; 5] = ["date", "datetime-local", "time", "month", "week"];

/// The ways code names the kind of a box: a JSX attribute, with or without braces, spread from an
/// object, and the DOM (by its attribute or its property).
const BOX_KINDS: [&str; 6] = [
    "type=\"",
    "type={\"",
    "{...{ type: \"",
    "setAttribute(\"type\", \"",
    ".type = \"",
    ".type=\"",
];

pub(crate) struct TxtDates;

impl Check for TxtDates {
    fn id(&self) -> &'static str {
        "TXT-DATES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&context.sources(self.id(), Kind::Web)?))
    }
}

fn message(writer: &str) -> String {
    format!(
        "`{writer}` writes a date, a time or a length here. Write it with the shared helpers of web/app/src/shared/format.ts, which follow the person's Region (TXT-DATES, coding-standards §4)."
    )
}

fn box_message(kind: &str) -> String {
    format!(
        "A box of the browser for a `{kind}` writes the day in the format of the device, not the person's Region. Use a text box read and written by the shared helpers (`DayBox`, or `dayText`, `readDay` and `dayProblem` of web/app/src/shared/) (TXT-DATES, coding-standards §4)."
    )
}

/// The browser's own box for a day or a time that a line of code asks for, if it asks for one.
fn browser_box(code: &str) -> Option<&'static str> {
    BROWSER_BOXES.into_iter().find(|kind| {
        BOX_KINDS
            .iter()
            .any(|asks| code.contains(&format!("{asks}{kind}\"")))
    })
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources {
        if !path.starts_with(WEB) || !is_app_source(path) {
            continue;
        }
        let helper = HELPERS.iter().any(|folder| path.starts_with(folder));
        for (index, line) in text.lines().enumerate() {
            let code = code_of(line);
            let at = || format!("{path}:{}", index + 1);
            if let Some(kind) = browser_box(code) {
                findings.push(Finding::at(at(), box_message(kind)));
            }
            if helper {
                continue;
            }
            if let Some(writer) = WRITERS.iter().find(|writer| code.contains(*writer)) {
                findings.push(Finding::at(at(), message(writer)));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(path: &str, text: &str) -> usize {
        evaluate(&[(path.to_owned(), text.to_owned())]).len()
    }

    #[test]
    fn good_example_the_shared_helpers_the_packages_and_the_tests() {
        let call = "new Intl.DateTimeFormat(locale, { timeZone }).format(moment)";
        assert_eq!(found("web/app/src/shared/format.ts", call), 0);
        assert_eq!(found("web/packages/messages/src/index.ts", call), 0);
        assert_eq!(found("web/app/src/features/x/expiry.test.ts", call), 0);
        assert_eq!(found("web/app/e2e/region.spec.ts", call), 0);
        assert_eq!(found("web/packages/ui/src/Clock.tsx", call), 0);
        assert_eq!(found("web/tooling/bench/report.ts", call), 0);
        // A screen that calls the helper, and a comment that names `Intl`.
        assert_eq!(
            found(
                "web/app/src/features/history/ChangeEntry.tsx",
                "// not Intl.DateTimeFormat\n<span>{formatDateTime(first.at)}</span>"
            ),
            0
        );
    }

    #[test]
    fn bad_example_a_screen_that_writes_a_date_itself() {
        // Outside the app's own folder too: any code of the browser that is no helper.
        assert_eq!(
            found("web/app/main-extra.ts", "new Intl.DateTimeFormat(\"en\")"),
            1
        );
        let screen = "web/app/src/features/issues/FixedRows.tsx";
        assert_eq!(
            found(
                screen,
                "const day = new Intl.DateTimeFormat(\"en\").format(at);"
            ),
            1
        );
        assert_eq!(found(screen, "<span>{date.toLocaleDateString()}</span>"), 1);
        assert_eq!(found(screen, "const time = at.toLocaleTimeString();"), 1);
        assert_eq!(
            found(screen, "const both = new Date(x).toLocaleString(\"de\");"),
            1
        );
        assert_eq!(
            found(
                screen,
                "new Intl.RelativeTimeFormat(locale).format(-1, \"day\")"
            ),
            1
        );
    }

    #[test]
    fn good_example_a_day_in_a_text_box_and_a_word_that_names_a_day() {
        let screen = "web/app/src/features/issues/DateRows.tsx";
        assert_eq!(found(screen, "<TextField label={name} value={typed} />"), 0);
        // The kind of an operand of the search language is no box.
        assert_eq!(found(screen, "return { type: \"date\", date, span };"), 0);
        assert_eq!(found(screen, "if (first?.type === \"date\") {"), 0);
        assert_eq!(
            found(
                "web/packages/ui/src/TextField.tsx",
                "<Input type=\"text\" />"
            ),
            0
        );
        assert_eq!(
            found("web/app/e2e/dates.spec.ts", "<input type=\"date\" />"),
            0
        );
    }

    #[test]
    fn bad_example_the_browsers_own_box_for_a_day() {
        let screen = "web/app/src/features/issues/DateRows.tsx";
        assert_eq!(found(screen, "<input type=\"date\" value={day} />"), 1);
        assert_eq!(found(screen, "<Input type={\"datetime-local\"} />"), 1);
        assert_eq!(found(screen, "box.setAttribute(\"type\", \"time\");"), 1);
        assert_eq!(found(screen, "box.type = \"date\";"), 1);
        assert_eq!(found(screen, "<input {...{ type: \"week\" }} />"), 1);
        // In the shared helpers and the packages too: no helper may write a day the device's way.
        assert_eq!(
            found(
                "web/packages/ui/src/TextField.tsx",
                "<Input type=\"date\" />"
            ),
            1
        );
        assert_eq!(
            found("web/app/src/shared/DayBox.tsx", "<input type=\"month\" />"),
            1
        );
    }
}
