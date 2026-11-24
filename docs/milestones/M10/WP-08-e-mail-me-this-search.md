# WP-08 — *E-mail me this search*

**Wave 2 · Folder / branch:** `bugaboo.win4` / `WIN4` · **Model:** Opus (data leaving by e-mail, a scheduled job run as the person, keep and removal rules)
**Features:** QRY-07 (owner, server and screens). ADR-0033 and ADR-0034 amendments (WP-01); answers M10-Q16, Q20.

## Goal

On any saved search a person may open, *E-mail me this search* sets a schedule: on chosen weekdays at a chosen hour of their time zone, Bugaboo runs the search as them and sends them a plain-text e-mail with what it finds. The e-mail follows the organization's e-mail rules. A person sees and stops their schedules in *Notification settings*. A schedule stops when the person is switched off or can no longer open the search, and is deleted when the person is removed.

## Where

- **Owns:** `crates/bb-core/src/search_mail.rs` (new: the schedule, its limits), `crates/bb-app/src/search_mail/**` (new: set, change, stop, the job that sends), the e-mail's text in `crates/bb-app/src/notifications/mail.rs` (one more kind of e-mail, written by the one plain-text writer of ADR-0033), `crates/bb-store/src/search_mail.rs` (new), migration **68** (`crates/bb-store/src/migrations/m0068_search_mail.rs`: `search_mail_schedule`), the job kind `search-mail.send` (`bb_app::jobs::JobKind`), the setting `search-mail.allowed` (organization, on at the start) in `crates/bb-core/src/settings/catalog.rs`, `crates/bb-api/src/api/search_mail.rs` (new), the window *E-mail me this search* in the saved search's menu (`web/app/src/features/search/`, the menu entry and a new `SearchMailWindow.tsx`), the list *E-mailed searches* under *My settings › Notification settings* (`web/app/src/features/notifications/`), the keep entry and the removal part, `userDocs/notifications/e-mailed-searches.md` (new).
- **Shared files, one entry each:** `catalog.rs`, `migrations/mod.rs` and `list.rs`, `router.rs`, `JobKind`, the keep list, the walk of a person's data, `messages/en.json`, `paths.ts`.
- **Does not touch:** saved searches' record (read), notification rows of events.

## What is decided

- **To the person only** (M10-Q16): a schedule sends to the address of the person who set it, never to anyone else; there is no field for another address.
- **When:** chosen weekdays (at least one) and a whole hour, in the person's time zone (*Region*); the job finds the schedules due each hour and sends each as its own `search-mail.send` job, spread over the hour by a fixed offset per schedule so 500 people at 8:00 do not run at once.
- **What:** the search runs as the person at sending time, with their rights then (QRY-05); the e-mail holds the saved search's name, at most 50 issues by its sort (ID, title, status, a link), "and 120 more" when there are more (a count of what the person sees), and a link to the search and to the schedule. Plain text only, the rules of ADR-0033, decision 3: no picture, no attachment, the links first, control and direction characters removed. *Short e-mails* on: no titles and no search name, only IDs, statuses and links.
- ***Only when something is new or changed*** (on at the start): nothing is sent when no issue found is new or changed since the last e-mail of this schedule. The schedule keeps only the time of its last e-mail, never the issues.
- **Stops** (M10-Q16): the job sends nothing and pauses the schedule when the person is switched off, when the search no longer opens for them (deleted, unshared, no longer checks), or when e-mail or `search-mail.allowed` is off; the person sees "Sends nothing: you can no longer open this search" in their list (UX design §4.13). A paused schedule starts again only by the person.
- **Limits:** at most 20 schedules per person, counted inside the change. The link in the e-mail opens Bugaboo (sign-in needed); no token in the e-mail changes anything.
- **AI apps and agents** (M10-Q20): no tool sets a schedule; the routes take a session only.

## Tasks

1. Write the acceptance tests below.
2. The schedule, its limits; migration 68 on both databases; marks and the person link.
3. The hourly finder and the job: run as the person, the e-mail's text, *Short e-mails*, *Only when something is new or changed*, the pauses.
4. The setting `search-mail.allowed`; keep and removal (deleted at *Remove a person*; schedules of a deleted search deleted with it).
5. The window and the list, with pictures in the four themes on computer and phone, looked at against UX §1.1 before the push.
6. User docs; ADR-0033 and ADR-0034 *Enforced by* lines; architecture §12 and *Personal data in M10*.

## Acceptance tests (write first)

- `qry_07_a_schedule_sends_the_search_to_the_person_who_set_it_only` (both databases).
- `qry_07_the_search_runs_with_the_persons_rights_at_sending_time`.
- `qry_07_the_e_mail_is_plain_text_with_at_most_50_issues_and_how_many_more`.
- `qry_07_short_e_mails_leave_out_titles_and_the_search_name`.
- `qry_07_nothing_is_sent_when_nothing_is_new_or_changed`.
- `qry_07_the_schedule_runs_at_the_hour_of_the_persons_time_zone_across_summer_time`.
- `qry_07_a_switched_off_person_gets_nothing_and_the_schedule_is_paused`.
- `qry_07_a_search_that_no_longer_opens_pauses_the_schedule_and_sends_nothing`.
- `qry_07_with_e_mail_or_the_setting_off_nothing_is_sent`.
- `qry_07_at_most_20_schedules_per_person`.
- `qry_07_a_field_the_person_may_not_read_never_reaches_the_e_mail` (SEC-MATRIX, the e-mail column).
- `qry_07_e_mail_me_this_search_sets_and_stops_a_schedule` (browser, computer and phone).
- `priv_04_removal_deletes_the_persons_schedules`.
- `priv_03_download_my_data_holds_the_persons_schedules`.
- SEC-MATRIX: every schedule route, for every starting role and an access key, both databases.

## Checks

DATA-MIGRATIONS, DATA-MARKS, DATA-LOGS (no address in logs), ARCH-DB-PARITY, ARCH-SETTINGS, ARCH-EVENTS, ARCH-API-FREEZE, SEC-MATRIX, TEST-A11Y, THEME-ONLY, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES.

## Docs to update

`userDocs/notifications/e-mailed-searches.md` (new) and the README of the notifications docs; ADR-0033 and ADR-0034 amendments; ADR-0044 and ADR-0045 amendments; architecture §12 and *Personal data in M10*; the generated personal-data list.

## Review

Code review with the SSOT/DRY/ODR gate: one plain-text writer for every e-mail, the search run through the one search, the pause rule in one place. Security review: what leaves by e-mail, the link, a person who lost access. Privacy review: the e-mail against the issue page of the same person; keep and removal.

## Personal data

The schedule names its person and their choice of days and hour (marked, linked to the person). The e-mail sends issue data to the person's own address, outside Bugaboo: the user doc says so, *Short e-mails* limits it, and the record of processing (PRIV-07, M14) will list it. Deleted at *Remove a person*; in *Download my data*.

## Depends on

WP-01 merged. Saved searches of M5; e-mail of M4.
