# M5 — Search and lists: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M5). Rules: "Working in a folder", "Life of a work package" and "Definition of done" in [04-milestones.md](../../04-milestones.md), the [coding standards](../../coding-standards.md) (the SSOT/DRY/ODR review gate of §1 blocks a merge) and the [glossary](../../glossary.md).

M5 starts from M4 (`m4-done`, API v1 frozen: new routes and optional members are additions, nothing is renamed, removed or tightened). What exists:

- `bb-searchlang` is an empty crate; `bb-wasm` carries Markdown and the wording, 344 KB raw, against a budget of 150 KB with Brotli.
- The issue list is one project at a time, newest first, with a cursor and *Show more*; the only filter is "reported by me" for people without *See issues* (`bb-store/src/issues.rs`). It is not virtualized, always grouped by status group, and has fixed columns without *Labels* (M3-F51, F171, F177).
- No full-text index exists; people, handles and project names are found with `LIKE`.
- No saved search exists, and no `/s/<id>` address.
- *Quick search* (Ctrl+K) finds app pages and projects; issues and actions were left for M5.
- The many-issues bar takes up to 5,000 ticked issues by ID.
- `cargo xtask dataset sample` makes the sample (10,000 issues) through the use cases; `dataset large` and the nightly job do not exist (M3-F92).

Decided already: ADR-0005 (one grammar in `bb-searchlang`, Simple search edits the tree, `parse(print(tree)) == tree`), ADR-0016 (SQLite FTS5 and PostgreSQL `tsvector` behind one port, fed from the outbox, in the same statement as the permission predicates), ADR-0004 (visibility compiled into the SQL, the parser knows only readable fields), ADR-0022 (a kind whose names are hidden cannot be searched, sorted or grouped), ADR-0009 (search and how it is shown live in the address).

M5 runs in **four waves**. A wave starts when the previous one is merged to `main`. At most four builders run at once.

| Wave | WP | Name | Who | Folder / branch |
|------|----|------|-----|-----------------|
| 1 | WP-01 | Decision records, words, the security chapter, the UX gaps, the maintainer's answers | Lead | `bugaboo.win1` / `WIN1` |
| 1 | [WP-02](WP-02-search-language.md) | The search language: grammar, parser, printer, checker, suggestions, WebAssembly | Builder | `bugaboo.win2` / `WIN2` |
| 1 | [WP-03](WP-03-large-dataset-and-nightly-benchmarks.md) | The large dataset, the benchmarks and the nightly run | Builder | `bugaboo.win3` / `WIN3` |
| 1 | [WP-11](WP-11-no-fixed-paths.md) | No fixed paths: ask the operating system (added later, a rule of the maintainer) | Builder | `bugaboo.win4` / `WIN4` |
| 2 | [WP-04](WP-04-searching-on-the-server.md) | Searching on the server: the compiler, permissions, limits, the search routes | Builder | `bugaboo.win2` / `WIN2` |
| 2 | [WP-05](WP-05-full-text-search.md) | Full-text search on both databases | Builder | `bugaboo.win3` / `WIN3` |
| 2 | [WP-06](WP-06-saved-searches-on-the-server.md) | Saved searches on the server: stored, shared, a stable address | Builder | `bugaboo.win4` / `WIN4` |
| 3 | [WP-07](WP-07-search-bar.md) | The search bar: Simple and Advanced, suggestions, mistakes, *Save search* and *Share* | Builder | `bugaboo.win2` / `WIN2` |
| 3 | [WP-08](WP-08-issue-list.md) | The issue list: columns, sorting, grouping, spacing, the count line, long lists | Builder | `bugaboo.win3` / `WIN3` |
| 3 | [WP-09](WP-09-quick-search.md) | *Quick search*: issues, saved searches, projects, pages and actions | Builder | `bugaboo.win4` / `WIN4` |
| 4 | [WP-10](WP-10-sample-speed-and-exit-demo.md) | Sample organization part 4, people searching while others change, speed on 1 M issues, the exit demo | Builder | `bugaboo.win2` / `WIN2` |

Progress is kept only in [`docs/STATUS.md`](../../STATUS.md). The work package files are written in WP-01, after the maintainer's answers; WP-02 and WP-03 first, so wave 1 started while the lead wrote the rest.

## The packages in short

- **WP-01 (lead):**
  - Amendments: ADR-0005 (the final grammar, a hand-written parser, M5-Q7), ADR-0016 (language, which texts are indexed, M5-Q5, Q6), ADR-0009 (one library for long lists, M5-Q11).
  - A new decision record *Saved searches*: who owns, shares, changes and deletes them, and that a search always runs with the rights of the person who runs it (M5-Q1).
  - The numbers of SEC-17 (M5-Q10), the new permission, the glossary words, and the M5 section of the security chapter.
  - The UX design for what is not drawn yet: the list of saved searches and the *Share* window, the *Customize* panel, the help sheet of the search language, the search bar on a phone, the empty and the error states, the *Search* page across projects (M5-Q8).
  - The names of the dataset command made one (`11-testing.md` and `07-cross-cutting.md` disagree).
- **WP-02 (QRY-01, QRY-03, QRY-04):**
  - `bb-searchlang`: words, parser, tree, the printer that writes one canonical text, the checker that takes the fields a person may read, and suggestions with "Did you mean …".
  - A hidden field and an unknown field give the same message.
  - TEST-SEARCH-ROUNDTRIP: a property test that every tree prints and parses back to itself.
  - The browser gets it through `bb-wasm`, loaded only when a search bar opens, inside the size budget.
- **WP-03 (PERF-01, the measuring part):**
  - `cargo xtask dataset large`: 1 M issues, 500 people, 500 projects, written in bulk with valid history (M5-Q4), checked by the sample's own rules.
  - The benchmarks of PERF-01 on both databases in a release build, and PERF-BENCH. They run first against today's lists, so the starting point is known.
  - The nightly job on this computer (M3-F92) that runs them and keeps the results.
- **WP-04 (QRY-05, SEC-17, PERF-05):**
  - The compiler in `bb-store`: tree → one parameterized statement with the visibility predicates and field levels, on both databases.
  - Sorting and grouping by any readable field. Cursors that stay stable when many issues share a value (M3-F24).
  - The limits: time, size of a search, page size and the rate (M5-Q10).
  - Routes: a search across the projects a person sees, and a project's list as the same search with `within`. Every new list route counts its statements (PERF-N1).
  - The `search` entry point of SEC-MATRIX (M2-F65), and the indexes the compiled joins need (M3-F45).
  - The store follow-ups M4-F11 and F12, and M3-F68 and F120 where a list becomes a search.
- **WP-05 (ISS-13, QRY-04 `contains`):**
  - An index of titles, descriptions and comments, kept from the outbox, on SQLite and PostgreSQL alike (M5-Q5, Q6).
  - A hit never comes from a text the reader cannot see: a hidden comment, a hidden long-text field or a project the reader is not in.
  - The port the compiler of WP-04 calls. The shape of the port is agreed in WP-01, so both packages can run in the same wave.
- **WP-06 (QRY-06, CFG-02 stored):**
  - A saved search: the search, how it is shown (columns, order, widths, spacing, grouping, sorting) and a stable address `/s/<id>`.
  - Private, or shared with groups or projects, by the rules of M5-Q1. Receivers see only what they may see.
  - Each project's default list (M5-Q2).
  - How it is shown keeps room for board, timeline and calendar. Those views come with their milestones.
- **WP-07 (QRY-01, QRY-02 on screen):**
  - One search bar above every list. Simple search uses filter buttons, *+ Add filter* and *Any of*. Advanced search marks the words, suggests, and underlines mistakes as UX design §4.3 draws it.
  - Switching between the two never loses anything, and the search is kept in the address.
  - *Save search*, *Share*, the help sheet.
  - The date range editor (M3-F46) and finding an issue by its title for an *Issue* field (M3-F62).
- **WP-08 (CFG-02 on screen, PERF-04 lists):**
  - The list of UX design §4.2: *Customize › Columns* (with *Labels*, M3-F171), widths, sorting by a column header, *Group by* any readable field, *Spacing*, and the count line (M3-F177).
  - Only the rows that can be seen are drawn (M3-F51, M5-Q11).
  - *Tick all results* for the many-issues bar (M5-Q9).
  - The *Search* page across projects.
  - The status-group colours from one place (M4-F70).
- **WP-09 (CFG-08):**
  - *Quick search* finds issues by ID or by words of the title, saved searches, projects, pages and actions (M5-Q3), always filtered by what the person may see.
  - It is moved out of the first chunk if that chunk needs the room (M2-F18).
- **WP-10 (exit):**
  - Sample part 4: saved searches, private and shared, for every role.
  - The ten people run gains "searches while others change data" (`11-testing.md`).
  - The nightly run on 1 M issues meets PERF-01. The preview of 5,000 issues gets its budget (M3-F133), and the fixed cost per request is measured (M4-F137).
  - The M5 exit demo, and the user docs with numbers taken from the code (M4-F82).

## Why the waves are in this order

- **Wave 1** has the language and the measuring. Both depend on nothing else in M5, and everything after them does. The language is pure Rust without I/O, so it does not touch the files of WP-03.
- **Wave 2** is the server. The compiler needs the tree of WP-02. Full-text search and saved searches need the compiler's shape, but not its code: WP-01 writes the port of full-text search and the stored form of a saved search, so the three packages run side by side in different crates and tables.
- **Wave 3** is the screens. They need the routes of wave 2. The search bar and the list share the address, so WP-01 fixes its format and WP-07 owns it.
- **Wave 4** measures everything on the large dataset and runs the demo.
- **Exit waves** follow the exit reviews, as in M4. The lead plans them from the findings.

## File ownership

| Package | Owns |
|---------|------|
| WP-02 | `crates/bb-searchlang/**`, the search part of `crates/bb-wasm`, `web/packages/wasm` (search bindings) |
| WP-03 | `xtask/src/dataset*`, the benchmark code, `tools/checks/perf-*`, the nightly job |
| WP-04 | `crates/bb-store/src/search/**`, `crates/bb-app/src/search/**`, `crates/bb-api/src/api/search*`, the issue list route, new migrations for indexes |
| WP-05 | `crates/bb-store/src/text_index/**`, its outbox reader, its migration |
| WP-06 | saved searches in `bb-core`, `bb-app`, `bb-store`, `bb-api`; its migration |
| WP-07 | `web/app/src/features/search/**`, the address format of a search |
| WP-08 | `web/app/src/features/issues/` list files, `SelectionBars.tsx`, the list parts of `design/theme.kdl` |
| WP-09 | `web/app/src/features/shell/QuickSearch*`, `quick-search-*` |

Migrations get their numbers from the lead, in the order of merging.

## Follow-ups handled in M5

| Follow-up | Package |
|-----------|---------|
| M2-F65 (the `search` entry point), M3-F24, F45, F68, F120, M4-F11, F12 | WP-04 |
| M3-F92 (the nightly job), M3-F114 (the project list on the large dataset) | WP-03 |
| M3-F46, F62 | WP-07 |
| M3-F51, F171, F177, M4-F70 | WP-08 |
| M2-F18 | WP-09 |
| M3-F133, M4-F137, M4-F82 | WP-10 |
| M4-F104 (the purge in steps), M4-F152 (workflow drafts on the server) | the package that touches those files next; else the exit wave |
| M3-F121 (more than 1,000 issues for an access key), M3-F123 (an event for a new tag), M4-F95 (a spinner on a busy button), M4-F10 | not in M5 (M5-Q12) |

## Checks switched on or grown in M5

- **TEST-SEARCH-ROUNDTRIP** (planned → active, WP-02): print and parse round trip, run by proptest.
- **PERF-BENCH** (planned → active, WP-03): the benchmarks on the large dataset, run nightly, with a failure when a budget of PERF-01 is broken.
- **PERF-N1** grows (WP-04, WP-06): every search and saved-search route.
- **SEC-MATRIX** grows (WP-04): the `search` entry point. Each permission is tried through a search, including a hidden field used to filter, sort, group and search by text.
- **ARCH-DB-PARITY** grows (WP-04, WP-05): the same search gives the same issues in the same order on both databases.

## Questions for the maintainer

Each question has a proposal. Accepting all of them takes every proposal as written.

**Answered: every proposal was accepted.** WP-01 writes each answer where it belongs.

- **M5-Q1: Who may share a saved search.**
  - A saved search is private when it is made.
  - Sharing it with groups or projects needs a new permission, *Share searches*. Everyone but Guests starts with it.
  - Only the person who made it changes or deletes it; an Admin may delete any. Others may *Save a copy*.
  - A shared search always runs with the rights of the person who opens it. Two people may see different issues in the same search, and nobody sees more by being shared with.
- **M5-Q2: The columns of a list.**
  - The default columns are those of UX design §4.2: status, ID, type, title, labels, priority, assigned, changed.
  - Grouped by status, *Comfortable* spacing.
  - A Project lead may set the project's default list; everyone may save their own.
- **M5-Q3: What *Quick search* finds in M5.**
  - Issues by ID or words of the title, saved searches, projects, settings pages.
  - A fixed list of actions: *Create issue*, *Go to …*, *Notification settings*, *Sign out*.
  - Documents, timers and the assistant come with their milestones.
- **M5-Q4: Where 1 M issues are measured.**
  - `cargo xtask dataset large` writes the rows in bulk, not through the use cases, which would take days. The history it writes is valid and is checked by the same rules as the sample.
  - The nightly run is on this computer, at night, on SQLite and PostgreSQL, release build. Results are kept in `.work/` and a short summary in the repository.
  - The budgets of PERF-01 count on this computer until a reference server exists: issue page and saved search p95 under 100 ms, a complex search under 300 ms.
- **M5-Q5: Words in full-text search.**
  - No stemming: "crash" does not find "crashes" unless typed as `crash*`. The words are compared without accents and without case.
  - This works the same for German and English text, and SQLite and PostgreSQL give the same results.
  - Results with words in the title come first, then the newest.
- **M5-Q6: What full-text search covers.**
  - Titles, descriptions and comments, as ISS-13 says.
  - Own long-text fields can be searched by name (`notes contains "…"`), and only where the reader may read the field.
  - Files: their names only. What is inside a file is not searched in M5.
- **M5-Q7: The words of the search language.**
  - English words (`and`, `or`, `not`, `is`, `in last`, `sort by`), and field names as on screen, written as one word (`assignedTo`, `statusGroup`), as in QRY-01.
  - When the app gets other languages, the search text stays the same in all of them.
- **M5-Q8: Searching across projects.**
  - A *Search* page in the sidebar searches every project you can see.
  - A project's list is the same page with `within WEB` filled in.
- **M5-Q9: Many issues from a search.**
  - *Tick all results* ticks up to 5,000 issues, the limit of the many-issues bar. Above that it says to narrow the search.
  - The preview before a change stays as it is.
- **M5-Q10: The limits of a search (SEC-17).**
  - At most 2 seconds per search, then a message to narrow it.
  - At most 50 conditions; `connected to … within N steps` at most 5 steps.
  - Pages of at most 100 (as today).
  - At most 60 searches a minute per person, *Quick search* typing counted once per pause.
- **M5-Q11: One way to draw long lists.** The people list already draws only the rows that can be seen (React Aria). The issue list and every other long list use the same, and ADR-0009 and the tech stack, which name TanStack Virtual, are corrected.
- **M5-Q12: What stays out of M5.**
  - More than 1,000 issues for an access key (M3-F121) waits for automation (M12).
  - An event for a new tag (M3-F123) waits for a history of tags.
  - A spinner on a busy button (M4-F95) waits for the theme editor (M7).
  - Saved searches shown as board, timeline or calendar come with those views (M7, M10). M5 stores how a search is shown so they can use it.
