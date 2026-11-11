# WP-10 — People and warnings on the timeline, and the timeline on a phone

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** PLAN-10, PLAN-11, PLAN-12, PLAN-13 (on screen), UX-02 (the phone timeline). UX design §4.7 (*People on the timeline*, *People on the bars*, *The list of plan warnings*, *On the phone*); [ADR-0030](../../adr/0030-planning-with-people.md) and amendment M8; M8-Q12.

## Goal

The timeline shows who does the work and when they cannot: the person on each bar, *Group by person*, hatched away days, holidays and days off, *Plan by effort*, the card of a bar with its effort and the person's time, and the list of plan warnings with their ways out. On a phone the timeline is a list grouped like its rows, with dates, the person and a short bar over seven weeks, and nothing is dragged.

## Owns

Within `web/app/src/features/timeline/**` (from WP-09): the person on the bar and the marks of *Group by person*, the hatched days, the switch *Plan by effort*, the effort and the person's time on the card of a bar, the list of plan warnings (`warnings/`) with the previews of its ways out, and the phone list (`phone/`); the parts they need in `design/theme.kdl`; messages; the user docs of planning with people (`userDocs/planning/people-on-the-timeline.md`, new).

## What is decided

- **People as *Who sees people* allows** (ADR-0030, decision 1): the picture and first name at a bar's end come as `PersonRef`; *Role only* shows *"Someone (Team member)"* and offers no *Group by person*; *Hidden* shows nobody, and hatched days have no word. *Group by person* draws the copies in the rows of the other assigned people as an outline, with *"Planned with Lea's work times"* on the card and in the label for screen readers (amendment M8).
- **Days without work:** away days hatched and labelled *Away*, holidays hatched with their name, days off shaded like weekends; inside a bar a hatched gap, so the bar grows instead of hiding the gap (ADR-0030, decision 3).
- **The card of a bar** shows the effort, *"Estimate 8 days · ×1.25 = 10 work days · Lea's own estimate"*, with *Change* (the window of WP-07), and the person's time, *"Mon–Fri, 8 hours a day · 100 % in Mobile app"*, as far as *Work times* lets the reader see it.
- **The list of plan warnings** is UX design §4.7 as written: *Warnings (3)* in the toolbar, one sentence per warning, the ways out that fit each, a preview before *Move it after* and *Make it longer* save anything, *Give it to someone else…* through the usual person picker with nobody suggested, accepted warnings faded at the end with who accepted them, and the closing sentence *"Away times show only as Away. Bugaboo never suggests whom to give an issue."* The sentences come from the server (WP-06); the screen does not word them again.
- **The phone timeline** (M8-Q12): a list grouped like the computer's rows; each issue with its status icon, ID, title, start and due date, its person, and a short bar over seven weeks with today marked, in its status tint and priority edge. Tapping opens the card as a sheet with *Set dates…*, *Add dependency…* and *Open*; *Warnings (3)* opens the list as a sheet; *Not scheduled yet* is the last group. No drag on a phone.
- **No rule of the plan in the browser** (ADR-0030, amendment M8): the screen draws what the plan answers.

## Tasks

1. Write the acceptance tests below.
2. The person on the bar, *Group by person* and its marks, by *Who sees people*.
3. The hatched days and the switch *Plan by effort*.
4. The card of a bar with the effort and the person's time.
5. The list of plan warnings and its ways out, with their previews.
6. The phone timeline.
7. User docs; pictures in the four themes on computer and phone, looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `plan_10_a_role_only_reader_sees_someone_and_no_group_by_person` (browser).
- `plan_10_group_by_person_draws_the_other_assigned_people_as_an_outline_with_its_words` (browser).
- `plan_11_away_days_are_hatched_and_labelled_away_and_holidays_carry_their_name` (browser).
- `plan_12_plan_by_effort_doubles_the_bars_when_the_share_is_halved` (browser; the exit demo's step).
- `plan_13_each_warning_offers_its_ways_out_and_keep_and_accept_fades_it` (browser).
- `plan_13_move_it_after_shows_a_preview_before_anything_moves` (browser).
- `plan_13_give_it_to_someone_else_suggests_nobody` (browser).
- `ux_02_the_phone_timeline_is_a_grouped_list_with_dates_person_and_a_seven_week_bar` (browser, phone).
- `ux_02_on_a_phone_set_dates_changes_the_dates_and_nothing_is_dragged` (browser, phone).
- TEST-A11Y: the list of warnings and the phone list, keyboard and screen-reader labels.

## Checks

ARCH-PERSON-REF, TEST-A11Y, THEME-ONLY, STY-PARTS, STY-TOKENS, STY-BOLD, STY-CONTRAST, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES, ARCH-WEB-API, PERF-BUNDLE, LINT-TS.

## Review

Code review with the SSOT/DRY/ODR gate: the warnings' sentences from the server only, one person picker, the bar of the phone list drawn with the same parts and scale as the computer's. A **UX review** from the pictures in the four themes, on computer and phone, against UX design §4.7, §1.1 and §7.

## Depends on

WP-09 merged (the timeline component); wave 2 (WP-06, WP-07). WP-09 and WP-10 both touch `web/app/src/features/timeline/**`: WP-10 starts from WP-09's merged component and works in its own sub-folders.
