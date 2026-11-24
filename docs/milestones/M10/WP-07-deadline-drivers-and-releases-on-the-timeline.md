# WP-07 — Deadline drivers, and releases on the timeline

**Wave 2 · Folder / branch:** `bugaboo.win2` / `WIN2` · **Model:** Opus (rules of the plan on the server, sight of the reader, new warnings)
**Features:** PLAN-05 (owner); ISS-08 and PLAN-04 on the timeline. ADR-0043 amendment (WP-01); answers M10-Q8, Q9; follow-up M8-Q3 (the rest).

## Goal

On the timeline, the switch *Deadline drivers* outlines the chain of issues that sets the end of each goal and release with a date in the view, and of the view itself; any delay on it moves the end. The plan warns about impossible dependencies with *Loop* and *Cannot fit*. Releases stand on the timeline as diamonds, rows can be grouped by release, and an issue that ends after the release date of its release shows *Delay*. All of it is worked out on the server over the issues the reader sees.

## Where

- **Owns:** the plan in `crates/bb-app/src/plan/**` (a new `drivers.rs`; the warnings in `types.rs` and `ways.rs`; releases in `build.rs` and `read.rs`), `crates/bb-store/src/plan.rs` (the releases of the issues read), the timeline route's answer (`crates/bb-api/src/api/timeline*.rs`, optional members only), *Group by release* in the timeline's options (`crates/bb-core/src/saved_search.rs`, the timeline's group choices only), the screen parts in `web/app/src/features/timeline/**` (the switch, the outline, the release diamond, the two warnings and their ways out, *Group by release*, the phone list's marks), the parts of the outline and the diamond in `design/theme.kdl`, `userDocs/planning/timeline.md` and `people-on-the-timeline.md`.
- **Shared files, one entry each:** `saved_search.rs` (WP-04 and WP-05 add view kinds there in wave 1: rebase), `messages/en.json`, `design/theme.kdl` (its own parts).
- **Does not touch:** releases' record (WP-02: read), the board or the calendar.
- No migration expected: accepted warnings store their kind as a word (ADR-0030, migration 52). If a new kind needs a column, the lead gives out the next free number.

## What is decided

- **Deadline drivers** (M10-Q9): for each end in the view (each goal with a *Due date*, each release with a *Release date*, and the last day of the view) the chain of dependencies, with no slack, that ends on it: the issues whose delay by a day moves that end by a day. Worked out by the plan on the server (ADR-0030, amendment M8: the browser holds no rule of the plan), with plan by effort when it is on. **Only over the issues the reader sees** (PLAN-08): a chain through a hidden issue is not outlined, and nothing tells that one exists; two readers may see different drivers, as they may see different bars.
- **The switch** *Deadline drivers* in the timeline's toolbar, stored with the view's options; on, the bars of a chain are outlined in the accent color (UX design §4.7), the arrows between them are drawn stronger, and the card of a bar says "Sets the end of Mobile app 2.0". Color is never the only signal: the outline has a pattern in forced colours and a word on the card and for screen readers.
- **New plan warnings** (M10-Q9): *Loop* — "WEB-160 and WEB-161 wait for each other; neither can start." Way out: *Open the relation* (the issue page's relation, to remove it); *Keep and accept*. *Cannot fit* — "WEB-284 is due on 14 Nov, but what it waits for ends on 18 Nov." Ways out: *Make it longer* (shows the due date it needs; moves it only when the person confirms), *Keep and accept*. Both follow the rules of the other warnings: accepted ones stay faded, and come back when a date changes (ADR-0030).
- **Releases on the timeline** (M10-Q8): a release with a *Release date* is a diamond on that date with its name, like a goal's due date (ADR-0043, decision 7). *Group by release* groups rows by *Planned for release*, with *None* last. An issue planned for a release that ends after its release date shows *Delay* with the goal's ways out: "WEB-280 now ends 2 days after release 2.5."
- **The phone** (M8-Q12): the list shows the diamond of a release in its group's header and marks the drivers with a word ("Sets the end").

## Tasks

1. Write the acceptance tests below.
2. Releases in the plan's read: diamonds, *Group by release*, *Delay* for a release.
3. Deadline drivers on the server, over the reader's sight, with and without plan by effort.
4. *Loop* and *Cannot fit* with their ways out.
5. The screen: the switch, the outline, the diamond, the warnings, the phone; pictures in the four themes on computer and phone, looked at against UX §1.1 and §7 before the push.
6. The user docs; ADR-0043 *Enforced by* lines; architecture §12 (the drivers over the reader's sight).

## Acceptance tests (write first)

- `plan_05_the_chain_that_sets_the_end_of_a_goal_is_the_drivers` (both databases).
- `plan_05_a_delay_of_a_driver_by_a_day_moves_the_end_by_a_day_and_of_any_other_issue_does_not`.
- `plan_05_drivers_are_worked_out_with_plan_by_effort_when_it_is_on`.
- `plan_05_no_driver_chain_passes_through_an_issue_the_reader_cannot_see`.
- `plan_05_a_stored_loop_of_dependencies_shows_the_warning_loop`.
- `plan_05_a_due_date_before_what_it_waits_for_can_end_shows_cannot_fit`.
- `plan_05_make_it_longer_moves_the_due_date_only_when_the_person_confirms`.
- `plan_04_rows_group_by_planned_for_release_with_none_last`.
- `iss_08_a_release_with_a_release_date_is_a_diamond_on_the_timeline`.
- `iss_08_an_issue_that_ends_after_its_release_date_shows_delay`.
- `plan_05_the_switch_outlines_the_drivers_and_the_card_says_what_they_set` (browser, computer).
- `plan_05_the_outline_is_seen_in_forced_colours_and_read_by_a_screen_reader` (browser).
- `plan_05_the_phone_list_marks_drivers_and_release_diamonds` (browser, phone).
- TEST-A11Y: the timeline with drivers on, in the four themes.
- `expectStill`: switching *Deadline drivers* on moves no bar.

## Checks

TEST-A11Y, THEME-ONLY, STY-PARTS, STY-TOKENS, STY-CONTRAST, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES, ARCH-DB-PARITY, ARCH-API-FREEZE, SEC-MATRIX (the timeline route's new members), PERF-BENCH (the timeline scenario with drivers on), PERF-N1.

## Docs to update

`userDocs/planning/timeline.md` (deadline drivers, releases, the two warnings), `userDocs/planning/people-on-the-timeline.md` (warnings list); ADR-0043 amendment; architecture §12.

## Review

Code review with the SSOT/DRY/ODR gate: the drivers in the plan only, the new warnings as entries of the one list of warnings with their ways out, the diamond drawn by the goal's code. Security review: a reader with a hidden issue on the chain. A UX review of the pictures in the four themes against UX design §4.7.

## Personal data

None new. Drivers and warnings name issues; people stay as *Who sees people* says on the bars.

## Depends on

WP-02 merged (releases). The timeline of M8.
