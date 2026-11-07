# WP-07 — Runs undone in one step, and the marks of text written with AI

**Wave 2 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** AI-38, COMP-08, COMP-09. ADR-0029 decision 7, ADR-0013 (the mark), ADR-0007 (events); follow-ups M3-F121 (runs), M6-F35.

## Goal

Everything one session of an AI app or one attempt of an agent changed in Bugaboo is one run: a list of its changes that can be undone in one step. A change somebody made again afterwards is skipped and named, as in *Change many issues*. Every text an AI wrote carries a stored mark, *Written with the assistant*, *Written by an AI app* or *Written by an agent*, shown beside the text, kept in the history and given in the API and every export.

## Owns

`crates/bb-app/src/runs/**`, the run of an event and *Undo the run* in `bb-store` (`change_run` and its steps), the implementation of `RunStore` (shape from WP-01), the text mark (store column, API member, history, exports), `web/app/src/shared/written-by*` (the mark beside a text), `web/app/src/features/runs/**` (the run page), its migrations (numbers from the lead).

## What is decided (ADR-0029 decision 7, COMP-09)

- **A run** is a row of `change_run` with a kind (*Change many issues*, *App session*, *Attempt*), its actor (with the app or the agent) and its state. Every event made inside a run carries the run. A run of *Change many issues* stays as it is.
- ***Undo the run*** has a preview (*"14 changes go back, 2 are skipped: WEB-142's status was changed by Ben since"*) and an apply with the preview's version, made with the one long preview of *Change many issues* and the same step-wise job for a large run. It writes new events that put the old values back, marked as an undo of the run; it never deletes history. Who may undo: the run's person, and for an attempt the reviewer through *Throw away* (WP-08); each change is judged again as the person who undoes it.
- **The mark** is a value stored with each text that can carry it (the description, a comment, a long own field, a note of agent work): *none*, *assistant*, *app* (with the app's name), *agent* (with the agent). It is set from the actor and the channel of the change, never by the client. Editing a marked text by a person keeps the mark and the history shows who changed it.
- **Machine-readable:** the API gives the mark as a member of each text; every export carries it (COMP-09); the history shows it.

## Tasks

1. **Runs in the store:** the kind and the actor of a run, the run of an event, the list of a run's changes (paged, as the reader may see them), on both databases.
2. ***Undo the run***: preview and apply through the long preview and the step-wise job; the skipped changes named. While here: the leftovers of the busy write (M6-F35).
3. **`RunStore`** as WP-01 wrote it, so WP-05 and WP-08 use the real one after this merge.
4. **The mark:** the column, set in the one place that writes texts, the API member, the history's sentence, the exports that exist.
5. **On screen:** the mark beside a text (description, comment, own field, history) in the quiet style of UX §7; the run page (its changes, *Undo the run* with the preview); pictures in four themes on computer and phone, looked at against UX §1.1 and §7 before the push; the report names them.
6. **M3-F121:** a run of an agent judges each change again as the agent, so an agent key's large change goes through its limits (WP-09), not through a job as a person; the row is closed or moved with a reason.

## Acceptance tests (write first)

- `ai_38_a_run_is_undone_in_one_step_and_skips_later_changes`.
- `ai_38_an_undo_writes_new_events_and_keeps_the_history`.
- `ai_38_the_changes_of_a_run_show_only_what_the_reader_may_see`.
- `ai_38_an_undo_is_judged_again_as_the_person_who_undoes_it`.
- `comp_09_a_comment_written_by_an_agent_carries_the_mark_in_the_api_and_the_history`.
- `comp_09_the_client_cannot_set_or_remove_the_mark`.
- `comp_08_the_mark_shows_beside_the_text` (browser).
- `ai_38_the_run_page_previews_and_undoes_a_run` (browser).
- SEC-MATRIX: the routes of runs, both databases.

## Checks

ARCH-EVENTS, ARCH-VERSIONED, ARCH-DB-PARITY, DATA-MARKS, PERF-N1, SEC-MATRIX, TEST-A11Y, THEME-ONLY, STY-PARTS, TXT-TERMS.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: runs on the one `change_run`, the undo through the one long preview and job of *Change many issues* (no second preview), the mark set in one place. A security reviewer reads who may undo what. A UX reviewer looks at the mark and the run page.

## Depends on

Wave 1 merged (WP-04: the actor in the event), WP-01 (the port `RunStore`). Merges first in wave 2.
