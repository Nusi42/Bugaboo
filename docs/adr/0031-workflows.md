# ADR-0031: Workflows: statuses as data, one judge for a status change

Status: **Accepted** · 2026-10-16 · refines WF-01…WF-06 and [ADR-0027](0027-layouts-and-rules-per-issue-type.md) decision 5; decisions M4-Q1, M4-Q2

## Context

Until M4 every issue type uses the starting workflow, a fixed enum in `bb-core` (`Status`: `open`, `in-progress`, `done`) in which any status may follow any other. The browser knows the same three words from the generated client. M4 lets an organization and its projects make their own workflows (WF-01), with rules on each status change (WF-03, WF-04), an editor (WF-02) and starter workflows (WF-06). ADR-0027 promised that this happens without a migration of stored issues.

## Decision

### 1. A workflow is data

- A **workflow** has statuses and status changes.
  - **Status:** a name, a status group, a position and a stored key.
  - **Status change:** a from-status, a to-status, the button text in words (*Start working*), and its rules.
- Workflows live in their own tables (migration of M4 WP-04). The key of a status is stable: renaming a status changes its name, never its key, so stored issues and history keep working.
- The three statuses of the starting workflow keep their keys `open`, `in-progress` and `done`. The starting workflow becomes the organization's *Simple* workflow, so no stored issue changes. Enforced by: a migration test on a database written by M3's code (M4 WP-04).
- The API sends a status as key, name and group. The browser keeps no list of statuses; it shows what the API sends. Enforced by: TXT-TERMS and review (no `statusWords` left in the client).

### 2. Which workflow counts

- A project and an issue type find their workflow like a layout: the nearest place with a row wins (the project, then its parents, then the organization); at one place, a row for the issue type wins over a row for all types (`bb_core::lookup::nearest_row`, ADR-0027 decision 2). There is no second lookup.
- A project may change the workflow it inherited; its sub-projects inherit the change (M4-Q2).

### 3. One judge for a status change

- One function in `bb-core` decides, for an issue, a person and a status change, whether the person may make it now and what it needs. Its answer is one of:
  - *allowed*;
  - *needs these fields* (*Ask for*, *Must be filled in*; enforced by: TEST-WORKFLOWS);
  - *not allowed, because …*, with a sentence.
- The issue page, the status button, *Change many issues* and its preview, the workflow editor's *Try it*, the API and later the assistant, apps and agents all use it. Enforced by: TEST-WORKFLOWS (M4 WP-04) and review (no second place that decides a status change).
- The rules:
  - **Who can do it:** roles, groups, *the person assigned*, *the person who reported it*.
  - **Only if …:** conditions on fields, in the same words as *Must be filled in*. Enforced by: TEST-WORKFLOWS.
  - **Must be filled in:** fields that need a value before the change. Enforced by: TEST-WORKFLOWS.
  - **Ask for:** fields shown in a small form during the change.
- The rules can only narrow what *Change issues* or *Change own issues* already allow; they never give a right. A field the person may not see is never asked for: then the change is *not allowed* with a sentence that names no hidden field. Enforced by: TEST-WORKFLOWS and SEC-MATRIX.
- *Then automatically …* (WF-05) and versions of workflows (WF-07) are V1.

### 4. Changing a workflow

- The editor works on a draft under the edit lock (ADR-0019). *Publish* makes it count.
- When a published workflow drops a status that issues still have, *Publish* asks where those issues go, and moves them as a run of many changes (ADR-0017), with history.
- The permission *Manage workflows* (organization or project; Admin everywhere, Project lead in their projects) is needed to change a workflow (M4-Q2). An access key may never hold it.

### 5. Starter workflows (M4-Q1)

| Workflow | Statuses (status group) | Used by |
|---|---|---|
| *Simple* | Open (Not started), In progress (In progress), Done (Done) | Task, Goal, and every issue type that exists at the upgrade |
| *Bug* | Open, In progress, Testing (In progress), Done, Won't fix (Done) | Bug, on a new organization |
| *Development* | Open, In progress, In review (In progress), Testing (In progress), Done | Feature, on a new organization |

- They are made once, from one definition in `bb-core`, as editable copies for the organization.
- An organization that ran M3 keeps *Simple* for every type until someone changes it, so nobody's issues move by themselves.
- The closed date still follows the *Done* group (ADR-0027 decision 5).

## Consequences

- Every place that knew the three statuses (the `Status` enum, `closed_after`, the DTO enums, `statusChoices` in the browser) changes to the data and the judge. That is most of M4 WP-04 and WP-07.
- *Change many issues* may now skip issues whose workflow does not allow the change, and its preview says so (UX §4.2).

## Alternatives considered

- **A workflow as a column on the issue type:** simpler, but projects could not change it, and it would be a second lookup next to layouts. Rejected.
- **Status IDs as numbers, names only in the browser:** the history and the API would need a join for every word. Rejected; the key is short text, as today.

## Enforced by

TEST-WORKFLOWS (new in M4 WP-04: every rule, on both databases), ARCH-EVENTS (every change of a workflow), ARCH-VERSIONED (the editor saves with a version), SEC-MATRIX (rows for the workflow routes), and review (one judge, no second lookup).

## Amendments (M4 WP-04)

What the first build of the server settled, so the later packages (WP-07 for the screens) can rely on it.

1. **No rows, no copy.** The tables are created empty by the migration. Where no place uses a stored workflow, the built-in *Simple* counts, so an upgraded database keeps every status and no issue moves. A new organization (no issues and no workflow at the first start) gets the three starter workflows and their uses: *Bug* for Bug, *Development* for Feature, *Simple* for all other types.
2. **Statuses are unique by name** in a workflow. A new status gets a key made from its name; a status change in a draft names a status by its key or by the name of a new one.
3. **The judge** (`bb_core::workflow::judge`) answers *allowed*, *needs these fields* or *not allowed* with one of four reasons: no such change, not for you, only if, cannot fill. Each reason has its own error code. A status that the workflow does not have (dropped, or after a move to another project) may go to any status of the workflow, without rules. A rule that names a field the person may not see turns the change down as *cannot fill* without naming it, before any value is looked at. *Needs* is the empty *Must be filled in* fields and the *Ask for* fields the request does not carry.
4. **Who may change a workflow.** The permission *Manage workflows* is checked at the place of the workflow: the project that contains all of its uses, or the organization. It is never held through an access key.
5. **Publish** plans the moves first (nothing is saved while an issue of a dropped status has no place to go), saves the workflow under the edit lock and the version, then moves the issues as many-changes in chunks, as the person who published; an issue that person cannot change stays where it is and can be put right by hand.
6. **Which roles may change a field here** (M3-F53) is one question of the policy (`bb_policy::roles_that_change`); the route `getWhoMayChange` answers it for the sentence of a refusal and for *Who can do it*. It names only roles somebody holds in the project, answers not found for a field the reader may not see, and names no role to a reader who is not shown the people of the project by role.
7. **Statement budget.** The workflows that count are read once for a request: three statements when places use stored workflows, one when none does. The ceiling of the reads of an issue page (PERF-05) went from 45 to 47 for them.
