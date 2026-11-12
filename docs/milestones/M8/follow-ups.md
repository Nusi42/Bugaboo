# M8 follow-ups

Points found while building M8 that were accepted for later. Each names where it will be done. The IDs (`M8-F01` …) are what the review refers to. WP-01 uses F01 and up, WP-02 F10 and up, WP-03 F20 and up, WP-04 F30 and up, WP-05 F40 and up, WP-06 F50 and up, WP-07 F60 and up, WP-08 F70 and up, WP-09 F80 and up, WP-10 F90 and up, WP-11 F100 and up, WP-12 F110 and up; rows found by the lead outside a package take F200 and up.

## From WP-02 (a green full run)

WP-02 brought a full run from five failing browser tests to one (2026-11-11): Chromium opens one connection per page instead of two, the start probe reuses its connections, the limit of calls of AI apps and the crowd of the concurrent test count on the clock of the app, and five waits were fixed at their cause. The maintainer let it land with the rest open (2026-11-11).

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M8-F10 | About one full run in two still fails one Chromium test at its first `page.goto` with `net::ERR_NO_BUFFER_SPACE`. It is not the connections in TIME_WAIT: with about 15,700 on the computer, 12,000 new connections still succeeded. Windows logged Tcpip events 4231 and 4266 (no ports left) on several days, once during a run | The bursts are shorter than a sample of two seconds; a finer look is needed | The lead: a network trace of Windows during one full run (the maintainer starts it as administrator), then the fix chosen from it, beside wave 1 |
| M8-F11 | WebKit opens about 47 secure connections per page that only shake hands and close; all requests go over one other connection. The WebKit part at the end of a full run peaks at about 1,050 connections in TIME_WAIT, so TEST-CONNECTIONS keeps its limit of 1,800 | Below 600 needs a choice: trust the test certificate in Windows, close such connections on the server without TIME_WAIT, or fewer WebKit pages at once | With M8-F10, from the same trace |
| M8-F12 | `cfg_08` of the quick search (an arrow key goes on from the search box, desktop) failed once: the saved searches and the issues come in one answer, and the focus is given in a `setTimeout` of `use-fresh-list.ts` | The order could not be made fixed in the test; the race may be in the app | With M8-F10 |
| M8-F13 | `iss_17` with three people on one lock (WebKit) once showed "ran out" instead of "taken over": the heartbeat may answer after a take-over (`edit-lock-session.ts`) | Product logic of the edit lock; not changed in a test package | With M8-F10 |
| M8-F14 | `qry_06 Save keeps a change of one's own search` (desktop) failed in the M7 exit runs, in no run of WP-02, and again in its landing run (2026-11-12, the search box never showed) | Not reproduced alone | With M8-F10, if it comes back |
| M8-F15 | The tests of time that start a work day or a timer some minutes back (`time-scene.ts` `scene`, `demo-m6-setup.ts` `startedTheDayEarlier`, an hour back) fail when a full run passes midnight: the start falls on the day before and the server answers `working-hours.not-a-work-day` (`time_01`, `ux_01` of the time sidebar, step 1 of the M6 exit demo, desktop and phone; the landing run of 2026-11-12, 00:34) | Found in the landing run of WP-02; nothing in WP-02 touched them | With M8-F10: the tests set the time of day on the clock of the app instead of counting back from the wall clock |
| M8-F16 | `ai_22 taking an app back ends what it may do at once` (desktop) found the page 2 pixels wider than the window (`expectNoSidewaysScroll`), and 4 pixels in the landing run of the dates, work times and own themes (2026-11-12) | Seen twice, so a real fault of the page *Connect apps* or of the test | With M8-F10: find what is wider than the window after an app is taken back |
| M8-F17 | `ux_4_2 beside the panel a filter chip does not wrap inside itself` (desktop, `issue-panel-parts.spec.ts:63`) read the box of an element while the page drew it again ("The element is not on the page"), the same kind of wait WP-02 fixed in `issue-list-end` with `roomUnderLastRow` | Seen twice: in the second landing run of WP-02, and in the landing run of the work-time screens (2026-11-12), there in `ux_4_2` at line 54 inside the helper `support/windows.ts:93` itself, so the helper's own read of the box races | With M8-F10: read the boxes in one step in the page, through the helper of `support/windows.ts` |

## From WP-03 (dates and dependencies)

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M8-F21 | The starting relation kinds have no mark of their own: migration 49 finds *blocks* by its place (the first kind, which never moves) | The place is stable; a real mark needs one more column | Only if a later package needs to find a starting kind again |
| M8-F24 | Layouts saved before migration 49 do not place *Start date* and *Due date*; the two wait under "Fields that are not on this form" | M3-F101 already says so for fields added later; the starting layouts place them | An administrator places them; no migration |
| M8-F25 | The sample has no small chain of dependencies with dates | Not needed by the server package | WP-12 (sample part 7) |
| M8-F26 | The loop check reads all dependencies of the kinds with a date effect once per change (one graph, built once) | Fine for one change; not measured on 1 M issues | WP-06 measures it before the plan calls it per row; WP-12 under load |
| M8-F27 | When one event moves both dates and changes a length (*Estimated time*), the history shows the length as plain text, not with its own look | The combined sentence replaces the lines of the event | WP-09, with the timeline's history |

## From WP-04 (work times on the server)

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M8-F30 | *Remove a person* must call `delete_work_times_in` inside the change that ends the membership; built and tested, not yet called | *Remove a person* comes in M9 | M9 |
| M8-F32 | `members_page` still reads people a page at a time and judges each | Fine at the sample's size | WP-12, under load |
| M8-F33 | The sum of shares counts only shares that were set; a person in a project with no share row has 100 % that the sum does not count | One rule now (`bb_core::work_times::over_100`) | WP-06 decides whether *Too much at once* counts unset shares |
| M8-F34 | Two away times may overlap or repeat; nothing merges them or warns | Not needed by the server package | WP-07, on the screen that enters them |
| M8-F35 | Done in WP-07: `plan_11_the_e_mail_of_changed_work_times_names_who_and_the_days` sends the e-mail and reads its text | — | — |
| M8-F36 | Done in WP-07: the glossary has *Afternoon only* / *Morning only*, and `userDocs/time/work-times.md` uses them | — | — |
| M8-F37 | `usual_work_day` repeats its version in all seven rows | A header table would be cleaner but is a second table | Only if it gets in the way |
| M8-F38 | A read of one person's work times reads the project tree twice (the statements stay the same for 2 and 20 projects) | Reusing the tree of `evaluate_projects` changes an interface | WP-06, if the plan reads it per person |

## From WP-07 (work times and the time factor on screen)

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M8-F60 | The time factor on the issue (the row *Time factor*, the window *How long will it take you?*, `plan_14_the_person_on_the_bar_says_takes_me_10_days_and_sees_their_own_estimate`, `plan_14_someone_else_sets_the_time_factor_and_the_issue_says_who`, `plan_14_no_list_offers_the_time_factor_as_a_column_or_filter`) is not built: the route of WP-06 did not exist yet | Built against the API of WP-06 when it is merged | WP-07, a second round after WP-06 |
| M8-F61 | Two schemas of the API description had the same name (`ShareBody` of saved searches and of shares): utoipa kept one, so the generated client sent the wrong body for `setShare`. Fixed by naming the second `ProjectShareBody`; nothing checks that every schema name is unique | A new check is a change of the checks (architecture §10, *Adding a rule*) | The lead: a test or check over the API description |
| M8-F62 | M8-F34 stays open: the window of an away time does not say when the days overlap another away time | Whether two away times overlap is a rule of work times; the server should answer it (coding-standards §4), with the refusal or a warning in the answer | WP-06 or the server's next round of work times |
| M8-F63 | The words of the 20 holidays and the 18 choices of *Holidays* are made by `words!` enums, so they go to the first page's part of the messages (PERF-BUNDLE stays green) | The rule of parts puts every made key in `shared` | Only if the first page runs out of room |
| M8-F64 | The help of the setting *Holidays* says "for you" also on *Work › Working hours*, where the organization's choice is set | One help text serves both levels in the registry | The words review of M8 |
| M8-F65 | `next_holidays` of the answer of work times is a list of three, named in `tools/checks/lean-exemptions.toml` beside `usual` (M8-F31) | Three holidays shown together need no pages | Only if the number grows |
| M8-F66 | *Undo* of a deletion that somebody with *Enter work times for others* made enters the away time again without the person's note: that reader never has the note, so it cannot be put back | Keeping the note needs the server to restore the away time it deleted (an *Undo* by its ID), not a new entry | The server's next round of work times |
| M8-F67 | *Holidays* on one's own page is the generic row of a setting, with *Reset to default* back to the organization's choice; UX §4.13 draws an option *As the organization (Germany)* inside the picker | One control for every setting (DRY); the picker would need the organization's value, which the answer of one's own settings does not hold | The maintainer, from the pictures: keep the generic row or ask for the option |
| M8-F68 | The mark of a sum over 100 % on *Project members* says "projects you can't see" without how many; UX §4.13 writes "…and 1 other project" | The server answers only whether there are such projects (`others_hidden`), never how many, so nothing about them leaks | The privacy review decides whether a count may be shown |
| M8-F69 | Two boxes type a day: `shared/DayBox.tsx` (the Region's format, used by the timesheet and the window of an away time) and `features/fields/DateBox.tsx` (the browser's date field, for *Start date*, *Due date* and own date fields), although the comment of `DateBox` says a day is typed one way everywhere | Merging them changes every date field of issues and its tests | A later clean-up step: one box for a day |
| M8-F200 | The shared choice chip (`ChoiceGroup`) shows which choice is selected by colour only (fill, border and text colour), against WCAG 1.4.1 | A cue that is not colour (a tick or a heavier edge) changes every choice of the app and its pictures | WP-11 (touch everywhere) or the UX pass of M8 |

## From WP-05 (own themes on the server)

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M8-F40 | Decided (the maintainer, 2026-11-12): the quiet (muted) text is darker in the built-in themes, Paper `#463f37` (was `#5a5148`) and Daylight `#464b5a` (was `#565d6b`); the hint text stays. The two roles must differ by a visible step: quiet and hint text are now about 1.4:1 apart instead of 1.1:1, and every pair stays readable (STY-CONTRAST). From M0-F03 | — | Done (WP-08 fix round) |
| M8-F44 | Decided: the maintainer accepted the larger budget of 166,000 bytes (2026-11-12). The module `bb-wasm` grew from 133.1 KB to 164.0 KB (Brotli) with the readability of an own theme, and its budget in `tools/checks/perf-bundle.toml` from 135,000 to 166,000 bytes. The rules of the base theme file are Rust data now (no reader of them in the module); what is left is about 9.7 KB for reading the values of a theme and about 21 KB for the rating with its rules and the colors of the built-in themes | Every page that previews Markdown or shows a history loads the module; a module of its own for the theme editor would keep them at their old size, but ADR-0026 (update M8) names `bb-wasm` | The maintainer: the larger budget as it is, or a module of its own for the editor (WP-08) |
| M8-F45 | Done (WP-08): the pickers of *My settings › Appearance* list the built-in themes, then the themes for everyone, then the person's own; *Look › Themes* the built-in themes and the themes for everyone; each own theme with a sample in its own colors. Was: *My settings › Appearance* and *Look › Themes* list only the built-in themes: an own theme that is chosen shows on the page, but no sample of the picker is marked. The server lists the own themes a person may choose (`GET /api/v1/themes`, with `choice` and `light`); that list and the pairs that are hard to read are small lists with a cap, named in `tools/checks/lean-exemptions.toml` until the editor shows whether they need pages | The screens of own themes are the theme editor's package | WP-08 |

## From WP-08 (the theme editor)

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M8-F70 | Decided (the maintainer, 2026-11-12): when a text color is changed, its color *If the device asks for more contrast* starts with the built-in theme's own color for more contrast, and it is rated as its own value | — | Done (WP-08 fix round) |
| M8-F71 | Settled: the readability leads with its verdict ("Readability: nothing is hard to read." or how many colors are hard to read, each on how many backgrounds); the counts of the combinations checked are behind *Details*, and no screen says "pairs" | — | Done (WP-08 fix round) |
| M8-F72 | The question before a theme for everyone is deleted counts the people who chose it themselves; people who follow the organization's choice of it are not counted (the sentence for nobody says that they see the built-in theme where it is the organization's choice) | Counting them needs the layers of every person | Only if an organization asks |
| M8-F73 | A shadow of its own starts with one thin layer (0, 1, 2, 0 pixels) in the color of the dimmed page; the editor has no picker of the shadow of the built-in theme, whose parts are CSS text in the theme file | Reading the built-in shadow as parts needs the generator to write it as parts | When a theme file's shadow is written as parts |
