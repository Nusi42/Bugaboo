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
3. **The judge** (`bb_core::workflow::judge`) answers *allowed*, *needs these fields* or *not allowed* with one of four reasons: no such change, not for you, only if, cannot fill. Each reason has its own error code. No rule is skipped by changing the type or the project: when a request changes either and the current status exists in the workflow that counts afterwards, a status change in the same request is judged in that workflow from the current status, with all its rules. When the current status does not exist there, the issue starts again in the starting status of that workflow, which is set when the request names no status, and any other status is refused. An issue that comes back from the trash, or whose project moves to another workflow, and whose workflow lacks its status starts again in the starting status, with history, inside that change. An issue that is outside its workflow in any other way may go only to the starting status; there is no free move. The one exception is the move of *Publish* for the issues of a dropped status, which the publisher sends where they chose (amendment 11). A rule that names a field the person may not see turns the change down as *cannot fill* without naming it, before any value is looked at. *Needs* is the empty *Must be filled in* fields and the *Ask for* fields the request does not carry.
4. **Who may change a workflow.** The permission *Manage workflows* is checked at the place of the workflow: the project that contains all of its uses, or the organization. It is never held through an access key.
5. **No change to workflows strands an issue.** One check, "every live issue that is governed after the change has a status of its workflow", runs on the places and workflows as they will be, inside the transaction of the change; *Publish*, a place that uses another workflow (or follows its parent again) and taking a workflow away all call it, and refuse with `workflow.status_in_use` when it fails. *Publish* judges every move of an issue of a dropped status first, with the rights of the person who publishes and as amendment 11 says; if any issue could not be moved, or there are more issues than amendment 12 allows, nothing is published. The moves are made inside the same change, each with its history, so nothing is answered as an error after the workflow was saved. The places the check looks at are read in the transaction, after the lock; *Publish*, switching a place, taking a workflow away, a change of where a type or a group is offered (ADR-0027) and retiring an issue type take the lock of the workflows alone, which waits for every holder of the shared one. Creating, changing and moving an issue, and putting a label on one, take it shared (`lock_chain_in`), before the lock on the codes of the project, and look at the workflow and the offers again, refusing with `edit.version_changed` if the status is gone. The one order of all locks is `LockClass` in `bb-store` and is kept by a guard in every transaction (ARCH-LOCK-ORDER). Issues in the trash keep their status; on restore amendment 3 applies. The issues to look at are found in SQL by the projects, types and statuses concerned, with indexes on project and status and on type and status, a page at a time (amendment 12).
6. **Which roles may change a field here** (M3-F53) is one question of the policy (`bb_policy::roles_that_change`); the route `getWhoMayChange` answers it for the sentence of a refusal and for *Who can do it*. It names only roles somebody holds in the project, answers not found for a field the reader may not see, and names no role to a reader who is not shown the people of the project by role.
7. **Statement budget.** The workflows that count are read once for a request: two statements when places use stored workflows (the uses come with the statuses), one when none does. The ceiling of the reads of an issue page (PERF-05) went from 45 to 46 for them, and to 47 with the step that reads the files of the issue (M4 WP-05; `perf_queries.rs`).
8. **A workflow's place never moves.** A place may use only a workflow that sits at that place or above it (the lowest project that holds all its uses, or the organization), and a workflow used nowhere is the organization's to hand out. The list of workflows for a place holds only those.
9. **Nothing of a hidden field or group leaks through the editor.** *Publish* refuses a new or changed rule that names a field the publisher may not see at the place or a group they may not know; rules that are there stay as they are. *Try it* answers *cannot fill*, with no field and no detail, for a change whose rules name a field the person who tries cannot see.
10. **Where a workflow sits can move down, never up.** The place of a workflow is the lowest project that holds all its uses. When the use at its top is taken away, the place moves down to the next one; no use can be added above it, because a place may use only a workflow at or above it (amendment 8). So a person who could manage it before can still manage it after.

## Amendments (M4 WP-07)

11. **Where the issues of a dropped status go.** *Publish* asks, for each dropped status that issues are in, which status of the new workflow they go to; any status of the new workflow is allowed, not only the starting status (M4-F42). The move is not a status change: it is judged with the rights of the person who publishes (they see the issue, may change it, and their level for *Status* is *Can change*) and with no rule of a status change (*Who can do it*, *Only if …*, *Must be filled in*, *Ask for*). It is made inside the same change as the publish, under the workflows lock and before the check of amendment 5, each move with its history and with the closed date following the group of the target. A move names its target by its key, or, for a status that is new in the draft, by its name, like a status change does (amendment 2); the server resolves it. If any issue is one the publisher cannot move, nothing is published. The rights are judged when the moves are planned, in the same way as in the other use cases, not again inside the transaction.
12. **What *Publish* tells before it asks.** The route `previewPublishWorkflow` (needs *Manage workflows* wherever the workflow is used, changes nothing, takes no lock) judges the workflow the editor would send with the code of *Publish* itself and answers the dropped statuses, and for each: how many live issues are in it, a few of them named as a list shows an issue, how many the person does not see, and how many the person could not move (M4-F43). Issues the person does not see are counted and never named or described. It also answers the most issues one publish moves (`MANY_CHANGES_ISSUES_MAX`); a publish that would move more is refused and the editor says so in plain words. The issues are read a page at a time and the reading stops one past `MANY_CHANGES_ISSUES_MAX`: a publish that would move more is refused with `workflow.status_in_use`, and the preview then says `too_many` and neither judges nor names issues. A job for a larger move is not built (M4-F45, decided with the lead: wait until an organization reaches the limit).
13. **Many issues carry the words of the status change.** *Change many issues* sends the status change that was chosen, its status and the words of its button. An issue is changed only when its own workflow has exactly that change from the status the issue is in, and is skipped with the reason *workflow* otherwise, also when its workflow has another change to the same status with other words. The list of the status changes of a project (`getProjectStatuses`) is the list of the choices, each status and words once.
14. **The description is never asked for.** A rule may require the description to be filled in, but not ask for it in the small form of a status change: it is long text with a lock of its own that a change cannot carry. `bb_core::workflow::ask_may_name` says so once; *Publish* refuses such a rule as `workflow.invalid`, and the editor offers *Ask for* only the fields the server marks as askable. A need for the description reaches the issue as the kind `description`, which the browser answers with a note to write it first.
