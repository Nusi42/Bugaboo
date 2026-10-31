# WP-03 — Dates, the time of day and the person's day

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** CFG-04 (*Region* in *My settings*), UX-07 (one way to write values for people); the ground for TIME-01, TIME-04 and TIME-08. Follow-ups M5-F137 (the dates and the time of day), M2-F72, M4-F132 (lengths of time). ADR-0033, amendment M4 WP-13 (the wording and its vectors); ADR-0005, decision 10.

## Goal

Every date, moment, time of day and length of time is written one way for a person, by their *Region*, everywhere: lists, chips, columns, the issue page, history, notifications, e-mails and, from wave 3, the timer and the timesheet. And before wave 2 stores any time, there is one answer to "which day is this moment for this person", one week, and one reader of a length of time that a person types.

## Owns

`crates/bb-core/src/time/**`; the date, moment and length parts of `bb_core::wording` and `cargo xtask gen word-vectors`; the Region settings in `crates/bb-core/src/settings/catalog.rs` and their messages; `web/app/src/shared/` helpers for dates and lengths (and the files that format dates today, only to call those helpers); the *Region* page of *My settings*; TXT-DATES in `tools/checks/catalog.toml` and `xtask/src/checks/`.

## What is decided (M6-Q2)

- *Region* gets **Time of day**: *24-hour (14:30)* or *12-hour (2:30 pm)*. Its default follows the date format: *Like my device* takes the device's clock; every fixed date format starts with *24-hour*.
- One helper writes each kind of value from the person's *Region*: a date, a moment (date and time of day), a time of day, a length of time. Rust (`bb_core::wording`) writes them for the server and the lazy parts; the synchronous TypeScript helpers are the approved exception of ADR-0033 and are held by the generated vectors (`tools/testdata/value-words.json`). No new exception is added.

## Tasks

1. **Time of day.** The setting `region.time-of-day` in the registry (choices, default rule, help text), the *Region* page with its preview (the preview shows a date and a time of day together, so the person sees the pair), and the vectors for both clocks with every date order.
2. **One form on screen (M5-F137).** Find every place that writes a date or a time (the issue list columns, chips, the issue page, history, notifications, e-mails, the settings pages, *Sessions*, access keys) and make it call the one helper. Where a column and a chip wrote one date two ways, they now agree; the report lists the places changed.
3. **TXT-DATES** ("Adding a rule", architecture §10): no `Intl.DateTimeFormat`, `toLocaleDateString`, `toLocaleTimeString` or `toLocaleString` of a date outside the shared helpers of `web/app/src/shared/` and `web/packages/`; bad and good examples.
4. **Time zones on the server (M2-F72).** The server checks a time zone against the list of zones it carries (`jiff` with its zone data, as `bb-core` already has under the feature `zone-data`; a new crate needs its register entry, DEP-REGISTER); an unknown zone is refused with a plain message. The browser offers the same list.
5. **The person's day** (`bb_core::time`, pure, no I/O): the date of a moment in a zone; the start and end of a date in a zone (23 or 25 hours on the days the clocks change); the week of a date by *First day of the week*; the days of a week. Property tests: across the daylight-saving changes of `Europe/Berlin` and `America/New_York`, a zone east of UTC+12, the 29th of February, a moment exactly at midnight.
6. **Lengths of time (M4-F132).** One reader of a typed length in whole minutes: `1:30`, `1.5` (with the decimal mark of the person's number format, so `1,5` in Germany), `90m`, `1h 30m`, `2h`; at most `TIME_ENTRY_MINUTES_MAX` for one entry; a plain mistake otherwise. One writer with the three forms of ADR-0033, amendment M6 (the grid form `1:30`, the short form `1h 30m` as today, and the words `1 hour 30 minutes` for sentences, which is new); UX §4.9 says which screen uses which. The bound of one entry, `TIME_ENTRY_MINUTES_MAX` (1,440 minutes), is defined once here in `bb_core::time`: the reader refuses more, and `bb-app::time` reuses the constant (ADR-0035, decision 1). The four places that turn days into lengths become one. The search language keeps its own words (ADR-0005, decision 10) and the same minutes.
7. **The browser** gets the reader through `bb-wasm` (lazy, as the wording), inside its budget (PERF-BUNDLE); the vectors hold the reader's cases too.

## Acceptance tests (write first)

- `cfg_04_the_time_of_day_follows_the_region_and_its_default_follows_the_date_format`.
- `ux_07_a_date_and_a_time_of_day_are_written_one_way_for_a_person_everywhere` (the vectors, and a browser test of a list column, a chip and a notification for the same moment).
- `time_08_the_day_of_a_moment_is_the_date_in_the_persons_zone_across_daylight_saving_changes` (property test).
- `time_04_a_typed_length_reads_the_same_in_every_form_and_respects_the_decimal_mark` (`1:30` = `1.5` = `1,5` (Germany) = `90m` = 90 minutes).
- The server refuses `Europe/Atlantis` with the message of an unknown zone.
- TXT-DATES fails on its bad example.

## Checks

TXT-DATES planned → active. DOC-GENERATED holds the vectors. PERF-BUNDLE for `bb-wasm`. LINT-TS and LINT-RUST as always.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: one writer per kind of value, one reader of lengths, one person's day; a date written by hand anywhere is a finding. A UX reviewer looks at the pictures of every screen whose dates changed. A words reviewer reads the Region page.

## Depends on

`main` at the M6 plan. It touches no file of WP-02 or WP-04.
