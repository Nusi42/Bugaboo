# WP-10 — The privacy screens: *My data*, *Keep and delete rules*, *Data exports*, *Remove a person*

**Wave 3 · Folder / branch:** `bugaboo.win3` / `WIN3` · **Model:** Opus (privacy words on screen)
**Features:** PRIV-03, PRIV-04, PRIV-05 (on screen). UX design §4.13 and §5.7 as WP-01 completes them; ADR-0044, ADR-0045.

## Goal

A person finds everything about their own data on one page: download it, see what is kept and for how long, and where each part is corrected. An administrator sees and changes the keep periods, deletes what is past its period, sees who made which export of whom, switches a person off and removes them after a window that says in plain words what happens.

## Owns

`web/app/src/features/my-settings/MyDataPage.tsx` (new: *Download my data* with its state and link, *What is kept and for how long* from the keep rules, *Where to correct it*); `web/app/src/features/settings/KeepRulesPage.tsx` (new: the table of two groups, a period changed with the count of what it would delete, *Delete them* per kind and per person, reusing `OldWorkDays.tsx` and `old-work-days.ts`, which become the general part for any kind of the second group); `web/app/src/features/settings/DataExportsPage.tsx` (new: the list of exports, *Download someone's data* with a person picker); `web/app/src/features/people/PersonState.tsx` and `RemovePersonWindow.tsx` (new: *Switch off*, *Switch on*, and the removal window with its three lists and the texts choice) on `PersonPage.tsx`; the addresses in `web/app/src/shared/paths.ts` and the menu entries in `SettingsScreens.tsx`; their texts in the message catalogue; `userDocs/privacy/` (new: `README.md`, `remove-a-person.md`, `data-exports.md`; the keep table of WP-04 is generated there).

## What is decided

- **Words and layout** come from WP-01's frames of UX design §4.13 and §5.7; this package adds no word the glossary lacks.
- ***My data*:** one button *Download my data*; while the file is made, the button says so and the page does not move (a state in place, UX §6.4); when it is ready, the notification and the page both offer the link for as long as the file is kept. *What is kept* lists each kind of the person's data with its period and who deletes it, read from `GET /api/v1/admin/keep-rules`' public part (a person may read the rules, not change them). *Accepted in review:* the notification that the file is ready offers *Open My data* rather than the download itself, because the signed address of a download lasts five minutes and a notification is read later; the page asks for a fresh address when the person presses *Download the file*.
- ***Keep and delete rules*:** one table, the two groups as two sections with a sentence each ("Bugaboo deletes these by itself", "An administrator deletes these after their period"); changing a period shows "This deletes 312 notifications tonight" before it saves; *Delete them* opens a window with the count per person and a confirmation, then shows the count deleted.
- ***Data exports*:** who made which export of whom and when, newest first, paged; *Download someone's data* with the person picker (people the administrator may see by name).
- **A person's page:** *Switch off* (a short confirmation that names what stops), *Switch on*; *Remove this person…* opens the window: what is deleted now, what stays shown as *Former user*, what is kept by law until which date (the end of the legal-hold name), the texts choice, and the button *Remove Malik Weber* (the person's name), disabled until the person is switched off (the window offers *Switch off first*). After the removal the page shows *Former user K7Q2* in place.
- **Permissions:** each control is shown only to holders of its permission; a refusal from the server is shown in plain words, never hidden.

## Tasks

1. Write the acceptance tests below.
2. *My data*.
3. *Keep and delete rules*, with the general part of *Delete them* made from the old-work-days one.
4. *Data exports*.
5. *Switch off*, *Switch on* and the removal window.
6. The user docs; pictures of every screen in four themes on computer and phone, and frame samples of each change made in place.

## Acceptance tests (write first)

- `priv_03 a person downloads their data from My data and the page does not move while it is made`.
- `priv_03 an administrator makes someone's export and it is listed in Data exports`.
- `priv_05 the keep table shows both groups and a changed period says what it deletes first`.
- `priv_05 Delete them deletes only what is past its period and says how many`.
- `priv_04 switch off names what stops and the person's agent stops`.
- `priv_04 the removal window says what goes what stays and until when`.
- `priv_04 after removal the person reads Former user in place`.
- TEST-A11Y: every new address in `paths.ts` measured; the windows keyboard-only.
- Each screen's pictures in four themes, computer and phone; frame samples for the export's state and the removal (WP-13 of M8's measure).

## Checks

TXT-LITERAL, TXT-CATALOG, TXT-TERMS, TXT-DATES, THEME-ONLY, STY-PARTS, STY-BOLD, ARCH-WEB-API, ARCH-WEB-FEATURES, LINT-DRY, TEST-A11Y, PERF-BUNDLE.

## Review

Code review with the SSOT/DRY/ODR gate: the general *Delete them* from the old-work-days one (no copy), the keep rules read from the server (no period written in the browser). **Privacy review** of the words: what the screens promise matches ADR-0044 and ADR-0045. **UX review** of the pictures in four themes, computer and phone, and of the frame samples.

## Depends on

WP-03, WP-04 and WP-06 merged; WP-09 merged before it lands (both touch `PersonPage.tsx`). Runs beside WP-11 and WP-13.
