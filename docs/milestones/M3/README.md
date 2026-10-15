# M3 — Issues, fields and editing: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M3). Rules: "Working in a folder", "Life of a work package" and "Definition of done" in [04-milestones.md](../../04-milestones.md), the [coding standards](../../coding-standards.md) (the SSOT/DRY/ODR review gate of §1 blocks a merge) and the [glossary](../../glossary.md).

M3 builds issues from nothing: no issue table, no issue permission, and empty `bb-markdown` and `bb-wasm` crates exist today. It runs in **four waves**. A wave starts when the previous one is merged to `main`. At most four builders run at once.

| Wave | WP | Name | Who | Folder / branch |
|------|----|------|-----|-----------------|
| 1 | [WP-01](WP-01-m3-docs.md) | Words, permissions, decision records and the maintainer's answers | Lead | `bugaboo.win1` / `WIN1` |
| 1 | [WP-02](WP-02-markdown-and-webassembly.md) | Safe Markdown on the server and in the browser (`bb-markdown`, `bb-wasm`) | Builder | `bugaboo.win2` / `WIN2` |
| 1 | [WP-03](WP-03-issues-core.md) | Issues, issue types, statuses and issue IDs | Builder | `bugaboo.win3` / `WIN3` |
| 1 | [WP-04](WP-04-versions-edit-locks-drafts.md) | Versions on every change, edit locks and drafts (server) | Builder | `bugaboo.win4` / `WIN4` |
| 2 | [WP-05](WP-05-own-fields.md) | Own fields: types, choices, rules, looks, permissions | Builder | `bugaboo.win2` / `WIN2` |
| 2 | [WP-06](WP-06-issue-page-and-new-issue.md) | Issue page, issue list and the new-issue form | Builder | `bugaboo.win3` / `WIN3` |
| 2 | [WP-07](WP-07-relations-comments-history-follow.md) | Sub-issues, related issues, comments, mentions, history, follow | Builder | `bugaboo.win4` / `WIN4` |
| 3 | [WP-08](WP-08-tags-labels-and-many-at-once.md) | Tags, labels, and changing many issues at once | Builder | `bugaboo.win2` / `WIN2` |
| 3 | [WP-09](WP-09-layout-editor.md) | Layout editor and role-based forms | Builder | `bugaboo.win3` / `WIN3` |
| 3 | [WP-10](WP-10-ten-people-at-once.md) | Ten people at once: TEST-CONCURRENT, TEST-EDIT-LOCKS, WebKit | Builder | `bugaboo.win4` / `WIN4` |
| 4 | [WP-11](WP-11-sample-organization-and-exit-demo.md) | Sample organization, part 2, speed, and the exit demo | Builder | `bugaboo.win2` / `WIN2` |
| 5 | [WP-12](WP-12-exit-security-fixes.md) | Exit review: security and privacy fixes | Builder | `bugaboo.win2` / `WIN2` |
| 5 | [WP-13](WP-13-exit-words-and-screens-fixes.md) | Exit review: words, screens and accessibility fixes | Builder | `bugaboo.win3` / `WIN3` |
| 5 | [WP-14](WP-14-exit-structure-fixes.md) | Exit review: structure fixes (one source for each fact) | Builder | `bugaboo.win4` / `WIN4` |

Progress (which package is building, merged, reviewed) is kept only in [`docs/STATUS.md`](../../STATUS.md).

## Before wave 1

The maintainer answers the questions at the end of this page (M3-Q1 … M3-Q10). WP-01 writes the answers into the docs. WP-02 and WP-04 need no answer; WP-03 needs M3-Q1 (statuses) and M3-Q6 (permissions) and starts with the rest.

## Why the waves are in this order

| Wave | Needs merged | Because |
|------|--------------|---------|
| 1 | M2C | WP-02 is pure crates and one browser component; WP-03 adds the issue tables and routes; WP-04 works on any versioned record and on M2's change routes. None of them needs another |
| 2 | WP-02, WP-03, WP-04 | Own fields hang on issues and projects (WP-03); the issue page shows Markdown (WP-02) and edits with locks and versions (WP-04); comments render Markdown and lock while being edited |
| 3 | WP-05, WP-06, WP-07 | Tags and labels are fields on the issue page; changing many issues needs the list and every field kind; the layout editor places own fields; ten people at once need every editing screen |
| 4 | everything | The sample organization is made through the use cases of all packages, and the exit demo uses the screens of all of them |

Inside wave 2, WP-06 owns the issue page. In its first pushed commit (on day one) it adds `web/app/src/features/issues/slots.ts`: one named slot for each block of the main column and the fields panel, with the props each block gets. WP-05 and WP-07 build their blocks as components in their own feature folders against that signature, register them in one added line each, and never edit WP-06's other files. Until WP-06 is merged they test their blocks on their own; the lead connects anything left at merge. Inside wave 3, WP-10 merges last: it runs the scenarios against the finished screens.

## File ownership

A package changes only what it owns; shared files get added lines only; the lead resolves conflicts at merge. The shared files of M3:

| File | Rule |
|------|------|
| `messages/en.json`, `docs/glossary.md` | Added lines only; new words go into the glossary first (WP-01 adds the M3 words) |
| `tools/checks/catalog.toml` | Each package changes only the entries of the checks it switches on |
| `docs/third-party/README.md` | One row per new dependency, in the same commit |
| `crates/bb-policy/src/catalogue.rs` | WP-01 adds every M3 permission with its starting values; packages only use them |
| `crates/bb-core/src/settings/catalog.rs` | Added settings only |
| `crates/bb-store/src/migrations/` | One numbered migration per package: WP-03 `m0007`, WP-04 `m0008`, WP-05 `m0009`, WP-07 `m0010`, WP-08 `m0011`, WP-09 `m0012`; later packages ask the lead |
| `crates/bb-core/src/event.rs` (`RecordKind`) | Added kinds only |
| `web/app/src/shared/paths.ts`, the menus of the shell | Added lines only |
| `web/packages/api-client/openapi.json` | Generated (`cargo xtask gen api`); a merge conflict is solved by generating again |
| `design/theme.kdl` | Added parts and tokens only, each package in its own block |
| `docs/architecture/12-security.md` | WP-01 writes the M3 section; packages add rows with the tests that prove them |
| `crates/bb-api/tests/sec_matrix*` (SEC-MATRIX record) | WP-03 adds the issues column; packages add rows for their routes |

## Follow-ups handled in M3

| Follow-up | Package |
|-----------|---------|
| M1-F10, M2-F03 (WebAssembly size budget, with `bb-wasm`) | WP-02 (moved here from M4: the Markdown preview is the first WebAssembly the browser loads) |
| M1-F23 (the outbox counter row makes changes wait for each other) | WP-11 (measured against PERF-01 on 10,000 issues; fixed if it misses the budget) |
| M2-F11 (`/frames` and `frames.*` are demo-only) | WP-06 (removed when the issue page uses the frames) |
| M2-F12 (e2e runs only in Chromium) | WP-10 (a WebKit phone project) |
| M2-F21 (`Masked` leaves nested views unmasked) | WP-03 (before the first nested view of an issue: a derive or a check) |
| M2-F26 (projects list and move preview read the whole tree) | WP-11, if the big sample shows it |
| M2-F63 (ARCH-NO-COUNTERS matches patterns, not meanings) | WP-03 (with counting issue IDs, the one allowed counter) |
| M2-F65 (SEC-MATRIX has no entry-point column) | WP-03 (the issues column) |
| M2-F70 (*Open issues in* is stored but never read) | WP-06 |
| M2-F96 (`sec_06` tests `fields=` only on people) | WP-03 (every field a role may not see, on issue answers) |
| M2-F99 (a Guest sees *Someone* only in the preview) | WP-06 |
| M2-F110 (the exit demo cannot show hidden *Cost center* or *Someone*) | WP-11 |
| M2C-F41 (`web/tooling/ports.ts` writes the block variable and folder prefix again) | WP-10 (it touches the test tooling) |
| M2C-F80 (the move preview runs to hundreds of lines) | WP-08 (one way to show a long preview, summary first, used by moving projects and changing many issues) |
| M2C-F81 (the focus view leaves a large empty area on a computer) | WP-06 |
| M2C-F100 (`ConfirmDialog` has an overlay of its own) | WP-06 (the new-issue dialog is the next dialog) |
| M2C-F103 (state tests cover only some parts) | each package that adds a state to a part |

**Not in M3:** M2-F81 (*correct the name*, belongs to a *Manage people* package), M2C-F101, F102, F104, F105 (looks of the shell and the design pages; when those files change next), M2C-F110…F112 (when SEC-PRIVATE is touched next), the tooling points META-SPEED under load and the doubled "E-mail address" label of the M1 screenshot spec (WP-10 takes the label if it touches that spec; otherwise a small lead fix).

## Checks switched on in M3

| Check | Package |
|-------|---------|
| ARCH-PURE (`bb-core`, `bb-searchlang`, `bb-markdown` do no I/O), SEC-PATTERNS, SEC-RICHTEXT (new: the sanitizer corpus) | WP-02 |
| ARCH-WASM with the size budget (PERF-04) | WP-02 |
| TEST-ISSUE-IDS | WP-03 |
| ARCH-VERSIONED | WP-04 |
| TEST-EDIT-LOCKS (server part) | WP-04; the browser part in WP-10 |
| TEST-CONCURRENT | WP-10 |
| TEST-SAMPLE-DATA (issues, fields, relations, comments) | WP-11 |

ARCH-EVENTS is already active: each package adds its commands to the table in `crates/bb-app/src/tests/events.rs`. ARCH-PERMS, ARCH-AUTHZ-ONLY, ARCH-API-OUT, ARCH-PERSON-REF, ARCH-LEAN, DATA-MARKS, SEC-MATRIX, TXT-TERMS and THEME-ONLY stay green for every new route, record and screen.

## What M3 covers of each feature

Some M3 features reach into later milestones. M3 builds the part that exists now and the machinery that later milestones plug into:

- **ISS-02, ISS-01 (status):** issue types with their icon, fields and layout. Workflows are M4 (WF-01): until then every type uses one starting workflow (M3-Q1).
- **ISS-03:** sub-issues and progress from the sub-issues the viewer can see; estimated time and time spent add up when time arrives (M6).
- **ISS-04:** relations from the issue page and for many issues at once; dragging on the timeline comes with the timeline (M7).
- **ISS-05, ISS-07, ISS-17:** mentions, followers, *Ask … to finish* and *Edit anyway* are stored and shown in the app; e-mail and in-app notifications come in M4 (M3-Q3).
- **CF-01:** every field type except *Release* (releases are ISS-08, V1) and *Pick one, then one more* (CF-02, V1).
- **CF-06, TAG-04:** own fields, tags and labels in the API, permissions and history now; in search (M5), the timeline (M7) and boards (M10) when those arrive.
- **CF-08:** own fields on issues and projects; time entries and working days (M6), documents (M11) and releases (V1) use the same machinery later.
- **CFG-01:** the issue page and the new-issue form; the board card comes with boards (M10); *See as …* is SEC-07 (M12).
- **ISS-09:** fields, status, labels and moving to another project (M3-Q4); more than 1,000 issues run as a background job.

## Later (not in M3)

Workflows and their editor (M4), notifications (M4), search and saved searches (M5), time (M6), the timeline and relation map (M7), restricted issues (SEC-05, V1), releases (ISS-08, V1), issue templates (ISS-10, V1), cloning and moving with field mapping (ISS-12, V1), internal comments (V1), look rules (CF-05, V1), live updates (ISS-14, V1; until then open issues are checked every 30 s), Tiptap (documents, M11).

## Decisions the lead makes in WP-01

These are technical and recorded in the decision records; the maintainer may still object:

1. **Version in the body.** Every change sends the version it read in the request body (`version`), as the M2 routes already do; ADR-0019 and ARCH-VERSIONED change from `If-Match` to the body. A list of many changes carries one version per item, which a header cannot.
2. **Own fields keep one key type.** The field catalogue's `Field` becomes a key that is either a built-in field or an own field (`FieldKey::BuiltIn(Field)` / `FieldKey::Own(FieldId)`); field levels, masks, the matrix and history all use the key, so own fields join every place built-in fields already go (ADR-0006, CF-06).
3. **Layouts and per-type rules.** Layouts and field levels per issue type are stored in their own tables, found like settings: the nearest project with a row wins, else the organization's; at one place a row for the issue type wins over a row for all types (a new ADR-0027).
4. **The Markdown editor** is a plain text box with a small toolbar and a preview rendered by the same `bb-markdown` in WebAssembly; Tiptap waits for documents (M11).
5. **The issue's stream of comments and changes** is called *History* (tabs *Everything · Comments · Changes*), because *Activity* already means the kind of work of a time entry (M3-Q5 asks the maintainer).

## Questions for the maintainer

Each question has a proposal. Accepting all of them takes every proposal as written.

**Answered: every proposal was accepted.** Where each answer is written:

- M3-Q1: [ADR-0027](../../adr/0027-layouts-and-rules-per-issue-type.md), decision 5, and the glossary (*Starting workflow*). The status groups keep the glossary's names *Not started*, *In progress*, *Done* (the question said "To do").
- M3-Q2: "What M3 covers of each feature" above.
- M3-Q3: [ADR-0019](../../adr/0019-edit-locks.md), amendment M3.
- M3-Q4: [ADR-0021](../../adr/0021-issue-ids.md), amendment M3; WP-08.
- M3-Q5: the glossary (*History*, *Combine with another choice*, *Retire*, *Pick one group*, *Keep Ben's / Use mine*), ADR-0019 amendment M3, UX §4.4.
- M3-Q6: the permission catalogue (`crates/bb-policy/src/catalogue.rs`) and the generated table in [architecture §7, *Starting roles*](../../architecture/07-cross-cutting.md#starting-roles).
- M3-Q7: WP-07, task 4; [security chapter, M3](../../architecture/12-security.md).
- M3-Q8: WP-08, tasks 1 and 3.
- M3-Q9: ADR-0019 amendment M3; the glossary (*Draft*).
- M3-Q10: WP-11, task 1.

The questions as they were asked:

- **M3-Q1 — Statuses before workflows.** Workflows come in M4, but an issue needs a status now. Proposal: every issue type starts with one starting workflow: **Open**, **In progress**, **Done** (groups *To do*, *In progress*, *Done*), and every status can follow every other. Moving into *Done* sets the closed date; leaving it clears it. M4's workflow editor replaces this without changing stored issues.
- **M3-Q2 — What M3 leaves for later.** CF-06, CF-08, TAG-04 and CFG-01 name search, boards, the timeline, time entries, documents and releases, which come in later milestones. Proposal: M3 builds own fields, tags and labels for issues and projects, in the API, permissions and history; each later milestone plugs them into its own screens (list above).
- **M3-Q3 — Mentions and following before notifications.** Notifications come in M4. Proposal: in M3, followers and mentions are stored and shown (*Followers 3 · Follow*, the *Mentions* block on the start page, unread first); *Ask Ana to finish* and *Edit anyway* show a note in Ana's open issue at the next heartbeat (within 15 s). E-mail and the bell come in M4 and send the same things.
- **M3-Q4 — Moving issues to another project.** ISS-09 and the list's bar have *Move to project*; moving with field mapping is ISS-12 (V1). Proposal: M3 moves issues, one or many, only into projects that have their issue type and own fields; the preview names the issues that would be skipped and why ("2 will be skipped: Handbook has no issue type Bug"). A moved issue gets a new issue ID and the old one keeps working.
- **M3-Q5 — Words.** (a) The issue's stream of comments and changes: proposal *History*, with tabs *Everything · Comments · Changes* (*Activity* stays the kind of work of a time entry). (b) Choices: *Combine with another choice* and *Retire* (UX §5.3), and the same two words for tags, labels and whole fields (CF-07, TAG-03: "merge", "archive"). (c) A label that excludes the others in its group: *Pick one* group. (d) Short-value conflicts use the UX's banner: "Ben changed Priority to Critical 2 minutes ago. [Keep Ben's] [Use mine]"; long text keeps *Compare and save*.
- **M3-Q6 — The issue permissions and who starts with them.** Proposal, new topics *Issues* and *Work settings*:

  | Permission | Admin | Project lead | Team member | Contributor | Viewer | Guest |
  |---|---|---|---|---|---|---|
  | See issues (all of the project's issues) | ✓ | ✓ | ✓ | ✓ | ✓ | — (only issues they reported) |
  | Make issues | ✓ | ✓ | ✓ | ✓ | — | ✓ |
  | Change issues (anybody's) | ✓ | ✓ | ✓ | — | — | — |
  | Change own issues (the ones they reported) | ✓ | ✓ | ✓ | ✓ | — | ✓ |
  | Comment | ✓ | ✓ | ✓ | ✓ | — | ✓ (on issues they see) |
  | Delete issues (to the trash, with Undo) | ✓ | ✓ | ✓ | — | — | — |
  | Move issues (at both projects) | ✓ | ✓ | ✓ | — | — | — |
  | Edit anyway (take over someone's editing) | ✓ | ✓ | — | — | — | — |
  | Manage issue types, kinds of relation and layouts | ✓ | ✓ (their projects) | — | — | — | — |
  | Manage fields | ✓ | — | — | — | — | — |
  | Manage labels and tags (also rename, combine) | ✓ | ✓ (their projects) | — | — | — | — |

  Access keys may never hold *Edit anyway* or the three *Manage* permissions. What a person may change inside an issue still follows the field levels (*Hidden / Can see / Can change*).
- **M3-Q7 — Who can be mentioned.** Proposal: mentions follow the kind of people information *Mentions*: a person who sees names may mention people and groups; a person who sees roles only (Guests) may mention a group or a role, never a person by name. Mentioning a group puts the comment into each member's *Mentions*.
- **M3-Q8 — Tags.** Proposal: anybody who may change an issue may add a tag; autocomplete offers only tags used in projects the person can see, so a hidden project's tags never show; a tag can be limited to some projects; renaming and combining tags or labels needs *Manage labels and tags* and is recorded in the history of every issue it touches (not the security log).
- **M3-Q9 — Drafts.** Drafts are private and kept on the server and in the browser (ADR-0019). Proposal: a draft is deleted when its text is saved or thrown away, and at the latest **30 days** after its last change; a draft for a new issue belongs to the person and the project, and the form offers "Continue your draft from yesterday" when it opens.
- **M3-Q10 — The sample organization.** The testing chapter plans restricted issues and releases in the sample organization, both V1. Proposal: leave them out of part 2 and add them with their features; everything else as planned (about 10,000 issues, own fields *Severity*, *Platforms* and *Customer*, about 3,000 relations, about 30,000 comments).
