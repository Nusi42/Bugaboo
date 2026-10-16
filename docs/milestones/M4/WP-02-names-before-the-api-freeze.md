# WP-02 — Names before the API freeze: *Create*, the code names, big files split

**Wave 1 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** ISS-09 (change many issues), TAG-01, TAG-02 (labels and tags), ISS-01a (issue IDs), SEC-02 (permission words); ADR-0008 (the API freeze at `m4-done`, M4-Q8); follow-ups M3-F141, M3-F152, M3-F153, M2-F93, M2C-F42.

## Goal

From `m4-done` on, renaming anything in `/api/v1` needs a decision record and a new version (ADR-0008 amendment, M4-Q8). Before that, every name in the code, the API, the stored values and the message keys says what the glossary says. Then TXT-TERMS reads code names too, so the drift cannot come back.

## Owns

Every file the renames touch, in all crates, `web/**` and `xtask/**`; migration `m0014` (stored names); `messages/en.json` (texts and keys: this package alone may change existing ones in M4); the TXT-TERMS check (`xtask`) and `tools/checks/terms-exemptions.toml`; the files split by task 4. The glossary rows are written by the lead in WP-01 (below); this package does not change the glossary except to add exemptions with reasons.

## The name map (WP-01, decided by the lead)

Each row is one commit or a few, never mixed with another row. "Stored" means a value written into the database, which `m0014` rewrites like `m0013` did for job kinds; old values must not be left behind.

| Today | New | Where | Why (glossary) |
|---|---|---|---|
| *Make …* in 34 texts (`sign-in.make-account`, `projects.make`, `roles.make`, `issue-types.make`, `fields.make`, `labels.make-label`, `access-keys.make`, `invitation.submit`, `shortcut.new-issue`, …; the list is in M3-F141 and the lead's research) | *Create …* in the text, and `make` → `create` in the key (`projects.create`, `roles.create-title`, …) | messages, every key use | **Create** (M3-F141) |
| Permission words `make-issues`, `make-projects`, `make-sub-projects`; sentences *Make issues* … | `create-issues`, `create-projects`, `create-sub-projects`; *Create issues* … | `bb-policy` catalogue, API words, **stored** (role and access-key permission rows), the generated roles table | **Create** |
| Rust and TypeScript names that say *make* for something new: `issues/make.rs`, `make_issue`, `MakeIssueTypeForm`, `MakeProjectForm` … | `issues/create.rs`, `create_issue`, `CreateIssueTypeForm` … | all code | **Create** |
| Route `/api/v1/issues/{issue}/sorting`, `IssueSorting`, `IssueSortingQuery` and the browser's names for it | `/api/v1/issues/{issue}/labels-and-tags`, `IssueLabelsAndTags`, `IssueLabelsAndTagsQuery` | API, client, screens | the screen says *Labels and tags* |
| Routes `/api/v1/bulk/changes`, `/bulk/preview`, `/bulk/runs/{run}`, `/bulk/runs/{run}/cancel` | `/api/v1/many-changes`, `/many-changes/preview`, `/many-changes/runs/{run}`, `/many-changes/runs/{run}/stop` | API, client | **Change many issues**, **Run**, *Stop* |
| `Bulk*` types and functions (`BulkRun`, `BulkPreview`, `BulkChange`, `ApplyBulk`, `bulk_title` …), modules `bb-app/src/bulk`, `api/bulk.rs`, `web/app/src/features/bulk` | `ManyChange*` (`ManyChangeRun`, `ManyChangePreview`, `ManyChange`, `ApplyManyChanges`, `many_changes_title` …), modules `many_changes`, `api/many_changes.rs`, `features/many-changes` | all code | **Change many issues** |
| Record kind `bulk_run`, job kind `bulk.step`, message keys `bulk.*` | `change_run`, `many-changes.step`, `many-changes.*` | **stored** (events, jobs), messages | **Run** (of a change to many issues) |
| `RunProgress` (browser) | stays | — | **Run** now has the meaning |
| `IssueKey`, `InvalidIssueKey`, `issue_key.rs`, `issue-key` in names | `ReadableId`, `InvalidReadableId`, `readable_id.rs`, `readable-id` | all code, API schema names | the glossary says *Issue ID* and bans *issue key*; `IssueId` stays the stored key that never changes |
| Table `issue_assignee`, `IssueAssignee*`, `clean_assignees`, `AssigneePicker` | table `assigned_person`, `AssignedPerson*`, `clean_assigned_people`, `AssignedPicker` | **stored** (table rename), all code | the glossary bans *assignee* |
| Keys `labels.field-label-group`, `labels.no-group`, `labels.make-group`, `labels.make-group-title`, and texts that say a bare *Group* for labels | `labels.field-label-group` (text *Label group*), `labels.no-label-group`, `labels.create-label-group`, `labels.create-label-group-title` | messages | **Label group** (a bare *Group* is people) |
| `LockNotes`, *edit notice* against *notes* | `EditNotices` (component and file); code keeps `EditNotice` | browser | internal term **Edit notice** |
| `KeyProjectRef` (API schema, M2-F93) | `ProjectLabelBody` | API | neutral name for a project that may be shown or hidden |
| `bb_app::status::DatabaseKind`, schema `DatabaseKindBody` (M2C-F42) | `bb_app::status::RunningDatabase`, `RunningDatabaseBody` | API, status page | two ideas had one name |

If a row turns out to touch something the map did not foresee (a stored value elsewhere, a name in the user docs), the builder stops and asks the lead instead of choosing a name.

## Tasks

1. **The rows of the map**, each as its own commit or commits, tests green after each. The OpenAPI document and the client are generated again (`cargo xtask gen api`). The user docs (`userDocs/**`) say the new words.
2. **Migration `m0014`:** rewrites every stored value of the map (permission words in role and access-key rows, the record kind in events and the outbox, the job kind, the table name), on both databases, with a test for each like `m0013`'s.
3. **TXT-TERMS reads code names.**
   - The check also reads the names this code declares in Rust and TypeScript: types, functions, modules, files, route segments, message keys.
   - It splits each name into words (`ManyChangeRun` → *many change run*, `issue_assignee` → *issue assignee*) and fails on a word from the *Don't say* column.
   - Names from other people's code (std, crates, npm) are not read.
   - The *Create* row's bracket changes so that *make* for something new fails in texts and names. A *make* in another sense ("make sure", `make_ascii_lowercase`) goes into `terms-exemptions.toml` with its reason.
   - The check's bad and good examples, as "Adding a rule" in architecture §10 asks.
4. **Big files split (M3-F153).**
   - Split `bb-api/src/api/tags.rs` (about 1,100 lines), `editing.rs`, `fields.rs` and every other source file over 800 lines into modules by job.
   - Behaviour flags passed as `bool` become small enums.
   - Store ports take domain types, not loose strings and numbers.
   - Each split is its own commit, with no change of behaviour.
5. **Everything stays the same for people.** No screen changes except the words of task 1. The showcase pictures differ only in those words.

## Acceptance tests (write first)

- `m0014_*`: each stored rename on both databases, from a database written by M3's code (a fixture made with `m3-done`), and running it twice changes nothing.
- TXT-TERMS: a Rust type `IssueAssignee`, a TypeScript function `makeProject` and a route `/bulk/x` each fail with a sentence that names the glossary row; `make_ascii_lowercase` from std does not fail.
- `sec_02_starting_roles_have_the_m4_permission_words` (the generated roles table matches the catalogue).
- The whole test suite, the SEC-MATRIX and the browser tests green under the new names; no test is deleted to make a rename pass.

## Checks

TXT-TERMS grows (task 3). All others stay green.

## Depends on

WP-01: the name map above and the glossary rows *Create*, **Run** (of a change to many issues), **Label group**, internal terms *Readable ID*, *Edit notice*. WP-04, WP-05 and WP-06 wait for this package, so they write new code under the new names.
