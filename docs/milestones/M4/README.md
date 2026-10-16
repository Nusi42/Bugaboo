# M4 — Workflows, files and notifications: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M4). Rules: "Working in a folder", "Life of a work package" and "Definition of done" in [04-milestones.md](../../04-milestones.md), the [coding standards](../../coding-standards.md) (the SSOT/DRY/ODR review gate of §1 blocks a merge) and the [glossary](../../glossary.md).

M4 starts from M3 (`m3-done`): issues have a fixed starting workflow (`bb_core::issue::Status`, three statuses, any change allowed), followers and mentions are stored but nothing is sent, and `bb-files` and `bb-notify` are empty crates. The outbox has one reader (history) and the job runner four kinds of job. No decision record covers files or notifications yet.

M4 runs in **four waves**. A wave starts when the previous one is merged to `main`. At most four builders run at once.

| Wave | WP | Name | Who | Folder / branch |
|------|----|------|-----|-----------------|
| 1 | [WP-01](WP-01-m4-docs.md) | Words, permissions, decision records, the name map and the maintainer's answers | Lead | `bugaboo.win1` / `WIN1` |
| 1 | [WP-02](WP-02-names-before-the-api-freeze.md) | Names before the API freeze: *Create*, the code names, big files split | Builder | `bugaboo.win2` / `WIN2` |
| 1 | [WP-03](WP-03-browser-tests-without-running-out-of-connections.md) | Browser tests that never run out of connections | Builder | `bugaboo.win3` / `WIN3` |
| 2 | [WP-04](WP-04-workflows-on-the-server.md) | Workflows on the server: statuses as data, status changes and their rules, starter workflows | Builder | `bugaboo.win2` / `WIN2` |
| 2 | [WP-05](WP-05-files-on-issues-and-comments.md) | Files on issues and comments: upload, safe storage, type checks, previews | Builder | `bugaboo.win3` / `WIN3` |
| 2 | [WP-06](WP-06-notifications-and-e-mail-on-the-server.md) | Notifications and e-mail on the server | Builder | `bugaboo.win4` / `WIN4` |
| 3 | [WP-07](WP-07-workflow-editor-and-status-changes-on-screen.md) | The workflow editor and status changes on screen | Builder | `bugaboo.win2` / `WIN2` |
| 3 | [WP-08](WP-08-notifications-on-screen-and-the-mail-server.md) | Notifications on screen, notification settings, the mail server | Builder | `bugaboo.win4` / `WIN4` |
| 3 | [WP-09](WP-09-issue-page-after-the-new-look.md) | The issue page after the new look (M3 follow-ups) | Builder | `bugaboo.win3` / `WIN3` |
| 4 | [WP-10](WP-10-sample-organization-and-exit-demo.md) | Sample organization part 3, ten people with workflows, speed, the exit demo | Builder | `bugaboo.win2` / `WIN2` |

Progress is kept only in [`docs/STATUS.md`](../../STATUS.md). The work package files were written in WP-01, after the maintainer's answers.

## The packages in short

- **WP-01 (lead):**
  - Three new decision records: ADR-0031 *Workflows*, ADR-0032 *Files*, ADR-0033 *Notifications and e-mail*.
  - An amendment to ADR-0008 that says when the API freeze is (M4-Q8).
  - New glossary words, with the name map for WP-02 written first.
  - The new permissions with the roles that start with them.
  - The M4 section of the security chapter.
  - The maintainer's answers.
- **WP-02:**
  - The 34 texts that say *Make …* say *Create …* (M3-F141).
  - Code names follow the glossary, by the map of WP-01 (M3-F152). This includes `/issues/{issue}/sorting`, `IssueSorting*`, `bulk`/`BulkRun`, `IssueKey`, *assignee*, the bare *Group* of labels and `LockNotes`. API names: M2-F93 and M2C-F42.
  - TXT-TERMS then also reads Rust and TypeScript names, so the drift cannot come back.
  - Big files split, behaviour flags as enums, store ports with domain types (M3-F153).
  - This package touches files everywhere, so it runs alone with WP-03, before the feature packages.
- **WP-03:** the first CI run of each of the last three merges lost one browser test to `ERR_NO_BUFFER_SPACE`. On Windows this means the computer ran out of free network ports. The likely causes:
  - a fresh server for every test file, on every worker;
  - a readiness probe every 100 ms;
  - new connections from every helper.

  WP-03 measures the open connections during a run, removes those causes, and adds a test that fails when a run leaves too many connections behind. It also makes the showcase pictures the same on every run (M2C-F104).
- **WP-04 (WF-01, WF-03, WF-04 server, WF-06):**
  - A workflow is data: statuses, each in a status group, and the status changes allowed between them, each with its button text ("Start working").
  - Each project and issue type finds its workflow like a layout (nearest place wins, ADR-0027).
  - The rules on a status change: *Who can do it*, *Only if …*, *Must be filled in*, *Ask for*.
  - The starter workflows *Simple*, *Bug* and *Development*. The starting workflow becomes *Simple*, and stored issues keep their statuses.
  - Changing many issues follows each issue's workflow, and the preview names the issues that will be skipped.
  - Issue types, labels and moving per project (M3-F22, F122, F124), and the answer to who may change a field (M3-F53), which the editor's *Who can do it* needs too.
- **WP-05 (ATT-01, ATT-02, ATT-03, ATT-04, ATT-07):**
  - **Upload** by dragging, pasting or choosing, on issues and comments; an interrupted upload goes on where it stopped.
  - **Storage** by content hash outside any web root.
  - **Type check** of the real type from the content.
  - **Limits** per project.
  - **Downloads** only through short-lived signed links, checked against the issue every time. SVG and HTML are never shown, only downloaded.
  - **Previews** of images and PDFs are made on the server in a separate process with limits on size, pixels and time, so a harmful file cannot hurt the server.
  - **Screens:** a *Files* block on the issue page and files on comments.
- **WP-06 (NTF-01, NTF-02 server):**
  - A second outbox reader turns changes into notifications for each person. It checks again, at the moment of sending, that the person still sees the issue and the field (M3-F75, F132).
  - Changes to things you follow, mentions, assignments, *Ask … to finish*, *Edit anyway*, and the end of a run of many changes (M3-F125, F150).
  - E-mail through the organization's mail server, as a job, with the same filter, the mail password stored encrypted (SEC-16), and off until an administrator switches it on (PRIV-08).
  - The e-mail column of SEC-MATRIX (M2-F65), and the keep-and-delete run for notifications (PRIV-05).
- **WP-07 (WF-02, WF-04 screens):**
  - The workflow editor as a plain list (and a drawing view on a computer), with drafts, *Publish*, *Try it* and the edit lock (UX §5.4).
  - The status button lists the status changes in words (M3-F175, F181).
  - *Ask for* opens a small window with only the fields it needs.
  - The many-issues bar follows the workflow.
- **WP-08 (NTF-01, NTF-02 screens):**
  - *Notifications* in the menu and on the phone's bottom bar, grouped by issue, with *Open* and *Mark as read* (UX §4.11).
  - *Customize* for each kind of event: in the app, by e-mail or off.
  - *Organization settings › Connections › E-mail & notifications* with *Send me a test e-mail*, and the optional e-mail step of the first start.
  - Once mail exists, signing up confirms the e-mail address (M1-F38).
- **WP-09:** the M3 follow-ups on the issue page:
  - *Labels and tags* as name-and-value rows (F179) and one `<dl>` (F188).
  - The priority edge on sub-issue rows (F173).
  - The facts bar following the layout (F102).
  - A field put back on a layout keeps its roles (F107).
  - A closed window lets its edit lock go at once (F105).
  - A refused save names who took over at once (F176, F180).
  - No empty values in the history at creation (F185).
  - The window note under the buttons (F186), and the smaller points F184, F187, F189.
- **WP-10:**
  - The sample organization grows through the use cases: workflows for each issue type, about 500 small files, notifications.
  - Contributors, Guests and Viewers do their part of the work (M3-F111).
  - The ten-people run with status changes under workflows.
  - Speed on the sample, and the exit demo on computer and phone.

## Why the waves are in this order

| Wave | Needs merged | Because |
|------|--------------|---------|
| 1 | M3 | WP-01 decides the words and the name map; WP-02 renames across the code and must land before new code adds more names; WP-03 touches only the test tooling |
| 2 | WP-02 | The three feature packages write new code under the new names; they share no tables (workflows, files, notifications) |
| 3 | WP-04, WP-05, WP-06 | The editor and the status button need the workflow rules; the notification screens need the notifications; WP-09 changes the issue page after WP-05 added its *Files* block |
| 4 | everything | The sample is made through the use cases of all packages, and the exit demo uses all screens |

Inside wave 3, WP-07 owns the status button and the *Ask for* window, and WP-09 owns *Details* and the side list. Both change the issue page only through `features/issues/slots.ts`.

## File ownership

As in M3: a package changes only what it owns; shared files get added lines only; the lead resolves conflicts at merge.

| File | Rule |
|------|------|
| `messages/en.json`, `docs/glossary.md` | Added lines only; WP-01 adds the M4 words; WP-02 alone may change existing texts and keys |
| `tools/checks/catalog.toml` | Each package changes only the entries of the checks it switches on |
| `docs/third-party/README.md` | One row per new dependency, in the same commit (`lettre`, `infer`, `image`, `hayro`: its license is checked first) |
| `crates/bb-policy/src/catalogue.rs` | WP-01 adds every M4 permission with its starting values |
| `crates/bb-store/src/migrations/` | WP-02 `m0014` (stored names), WP-04 `m0015`, WP-05 `m0016`, WP-06 `m0017`; later packages ask the lead |
| `crates/bb-core/src/event.rs` (`RecordKind`) | Added kinds only |
| `crates/bb-app/src/jobs.rs` (job kinds) | Added kinds only |
| `web/packages/api-client/openapi.json` | Generated; a conflict is solved by generating again |
| `design/theme.kdl` | Added parts and tokens only, each package in its own block |
| `docs/architecture/12-security.md` | WP-01 writes the M4 section; packages add rows with the tests that prove them |

## Follow-ups handled in M4

| Follow-up | Package |
|-----------|---------|
| M3-F141, M3-F152, M3-F153, M2-F93, M2C-F42 | WP-02 |
| M2C-F104 (showcase pictures differ between runs) | [WP-03](WP-03-browser-tests-without-running-out-of-connections.md) |
| M3-F22, F44, F53, F122, F124, F23 (*Priority* as a list stays: decided in WP-01) | WP-04 |
| M1-F30 (attachments get their own body limit; the row said M5) | WP-05 |
| M3-F75, F125, F126, F132, F150, F70 (a unique handle for mentions, decided in WP-01), M2-F65 (e-mail column), M1-F21, M1-F43, M1-F61 (jobs that keep and delete) | WP-06 |
| M3-F175, F181, F172 (other keys for resizing a window, if a test with a screen reader shows the clash) | WP-07 |
| M1-F38 (confirming an e-mail address at sign-up) | WP-08 |
| M3-F102, F105, F107, F173, F176, F179, F180, F184…F189, F100 (decided in WP-01) | WP-09 |
| M3-F93, F111, F113, F121 | WP-10 |
| M3-F154, F155 | the package that touches those files next (WP-04 and WP-06) |

**Not in M4:** M1-F49 and M1-F53 (a warning for a sign-in from a new network: M9, with operations), M2-F45 (only if the benchmarks show it), M1-F10 and M2-F03 (done in M3 WP-02; WP-01 marks them).

## Checks switched on or grown in M4

| Check | Package |
|-------|---------|
| TXT-TERMS reads Rust and TypeScript names, and *Make* for something new fails | WP-02 |
| A connection count in the browser test run (a new check, "Adding a rule" in architecture §10) | WP-03 |
| TEST-WORKFLOWS (new: every rule of a status change on both databases) | WP-04 |
| TEST-UPLOADS (new: wrong types, SVG, HTML, too large, a decompression bomb, a broken PDF) | WP-05 |
| SEC-MATRIX: the e-mail and notifications column | WP-06 |

ARCH-EVENTS, ARCH-VERSIONED, ARCH-PERMS, ARCH-AUTHZ-ONLY, ARCH-PERSON-REF, DATA-MARKS, DATA-LOGS, SEC-HEADERS, THEME-ONLY and TXT-TERMS stay green for every new route, record, job, e-mail and screen.

## Decisions the lead makes in WP-01

These are technical and recorded in the decision records; the maintainer may still object:

1. **Statuses are data, keys stay.** A workflow is stored in its own tables and found with `nearest_row` like a layout (ADR-0027). The three statuses of the starting workflow keep their stored keys `open`, `in-progress` and `done`, so no stored issue changes. The API sends a status as its key, its name and its group; the browser stops knowing a fixed list of statuses.
2. **One judge for a status change.** One function decides whether a person may make a status change on an issue now, and what it needs (*Ask for*). It is used by the issue page, changing many issues, the preview and *Try it*.
3. **Files by content hash.** A file is stored once by its SHA-256 under `files/`. Attachments point to it, and it is deleted when nothing points to it any more and no keep rule applies. Uploads go in parts, so an interrupted upload goes on where it stopped (Bugaboo's own simple protocol, no extra library).
4. **Previews in a separate process.** `bugaboo preview` runs as a child process with limits on memory, time and pixels, and writes only a small safe image. A file that crashes it gets no preview, and the server goes on.
5. **Signed links for downloads.** A download link is good for 5 minutes and for one person. Every download checks again that the person sees the issue.
6. **Notifications are an outbox reader.** Like history, a reader called `notifications` turns events into rows for each person. It checks visibility and the field mask when it builds the row and again when it is shown or sent. E-mail is a `mail.send` job through `lettre`.
7. **The workflow editor** is a lazy-loaded part of the web app (architecture §4); on a phone it is the plain list.

## Questions for the maintainer

Each question has a proposal. Accepting all of them takes every proposal as written.

**Answered: every proposal was accepted.** WP-01 writes each answer where it belongs.

- **M4-Q1: The starter workflows.**

  | Workflow | Statuses (status group) | For |
  |---|---|---|
  | *Simple* | Open (Not started), In progress (In progress), Done (Done) | Task, Goal, and every issue type that exists today |
  | *Bug* | Open, In progress, Testing (In progress), Done, Won't fix (Done) | Bug |
  | *Development* | Open, In progress, In review (In progress), Testing (In progress), Done | Feature |

  Each is an editable copy for the organization. Existing issues keep their status.
- **M4-Q2: Who may change workflows.** A new permission *Manage workflows*. Admins have it everywhere and Project leads in their projects. A project may change the workflow it inherited; the change counts for its sub-projects.
- **M4-Q3: Who may add and delete files.**
  - **Adding:** anybody who may change the issue (also Guests on the issues they reported), and anybody who may comment, for files on their comment.
  - **Deleting:** the person who added the file, or a person with *Change issues*. It goes to the trash with *Undo*, like issues.
- **M4-Q4: Size and type limits.**
  - **Size:** at most 25 MB per file. A project may set less, and the organization may set up to 100 MB.
  - **Types:** all types are accepted. Images and PDFs get a preview; everything else is only downloaded.
  - **Blocked list:** the organization may add types it never accepts (for example `.exe`).
- **M4-Q5: What people are told about by default.**

  | Event | In the app | By e-mail |
  |---|---|---|
  | You are mentioned | on | on |
  | You are assigned | on | on |
  | A status change, a comment or a file on an issue you follow | on | off |
  | Others' changes to fields of issues you follow | off | off |
  | Your run of many changes ended | on | off |
  | *Ask … to finish* and *Edit anyway* on your text | on | off |

  - You are never told about your own changes.
  - Everyone can change these rows under *Customize*.
- **M4-Q6: What an e-mail says.**
  - **Contents:** the issue ID and title, who changed what, and the comment text, all filtered by what the person may see, with a link into Bugaboo.
  - **Batching:** changes to one issue within 5 minutes go into one e-mail.
  - **Short e-mails:** the organization can switch to e-mails that leave out all issue text (title and comments). This is for places where mail leaves the company.
  - **No digest:** there is no daily summary in M4.
- **M4-Q7: Notifications and the *Mentions* block.** *Notifications* replaces the *Mentions* block on the start page. *Mentions* becomes a filter in Notifications, and the start page shows the newest unread ones. Notifications are kept 90 days (PRIV-05, as in UX §5.7).
- **M4-Q8: When the API freeze is.** At `m4-done`. From then on, renaming or removing anything in `/api/v1` needs a decision record and a new version (ADR-0008). Adding new routes and fields stays free. That is why the renames of WP-02 come first.
- **M4-Q9: A unique handle for mentions (M3-F70).** Every person gets a short handle (`@ana.brandt`), made from the name when the account is made and changeable in *My settings*. Mentions store the person, so a changed handle never breaks an old comment.
- **M4-Q10: Confirming e-mail addresses.**
  - **When mail is on:** signing up on one's own sends a confirmation link, valid for 24 hours, and the account works only after it (M1-F38).
  - **When mail is off:** signing up works as today, and the security chapter keeps the accepted risk that the address is not confirmed.
