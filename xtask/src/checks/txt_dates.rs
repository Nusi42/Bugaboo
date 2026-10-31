//! TXT-DATES: a date, a moment, a time of day and a length of time on a screen are written only by
//! the shared helpers (UX-07, CFG-04, coding-standards §4).
//!
//! The person's *Region* says how each is written, and one helper writes each kind
//! (`web/app/src/shared/format.ts` and its neighbours; the packages of `web/packages/`). A screen
//! that asks `Intl` itself, or a date for its `toLocale…String`, writes it a second way, which may
//! not follow the person's choices (a day first beside a clock of 12 hours, M5-F137). A finding is
//! one of [`WRITERS`] anywhere in `web/` outside the shared helpers ([`HELPERS`]), tests and tools
//! excluded (a test may work out what it expects another way).

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

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources {
        let helper = HELPERS.iter().any(|folder| path.starts_with(folder));
        if !path.starts_with(WEB) || helper || !is_app_source(path) {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            let code = code_of(line);
            if let Some(writer) = WRITERS.iter().find(|writer| code.contains(*writer)) {
                findings.push(Finding::at(
                    format!("{path}:{}", index + 1),
                    message(writer),
                ));
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
}
