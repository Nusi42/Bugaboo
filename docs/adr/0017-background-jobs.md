# ADR-0017: Background jobs stored in the database

Status: **Accepted** · 2026-10-04

## Context

Previews, virus scans, e-mail, webhooks, digests, backups, keep-and-delete runs and person removal need reliable background work.

## Decision

A small job queue in the database (`job` table with `run_at`, attempts, lease) run by `bb-jobs` inside the server process. Leases use `SELECT … FOR UPDATE SKIP LOCKED` on PostgreSQL and the single writer queue on SQLite. The jobs that come back every day repeat every 24 hours, fixed in the code (`bb_app::jobs::DAILY`); a schedule that people set (cron-like) becomes a setting in the registry when a job needs one (M9).

## Consequences

- Nothing extra to install; jobs survive restarts.
- Throughput is enough for the scale target; heavy work (previews) is limited by a worker pool size setting.

## Alternatives considered

- **Redis/RabbitMQ:** more installation and attack surface. Rejected.

## Enforced by

review; every job type has an idempotency test (part of ARCH-EVENTS test harness) (see `tools/checks/catalog.toml`).

## Amendment (M3 WP-08)

- **The runner exists (`bb-jobs`).** It depends on `bb-app` only (ARCH-DEPS): `spawn(services, every)` calls `Ctx::run_due_jobs()` on a timer, and `bb-server` starts it. The queue is behind the port `JobStore`; a round takes up to five due jobs under a lease of 120 seconds, runs each, reports it done (it leaves the queue) or failed, and a job that did a step queues the next one when more is left. A job that failed five starts goes to the dead-letter list.
- **Kinds so far:** `bulk.step` (changing many issues: 100 issues in a step, progress kept in the table `bulk_run` after each issue, so a step that stops half way goes on at the next issue and no issue is changed twice) and `issues.renew-ids` (ADR-0021). `bulk.step` judges again at run time what it does to each issue, with the permissions of the person who asked (`Ctx::acting`, `ctx_acting_as`). `issues.renew-ids` judges nothing: it finishes a change that was judged when the person changed the mode of IDs (they needed the right to change the project's settings then), so it renews the IDs of every issue of the project that does not have the new mode yet, and names that person on the events. Both are idempotent.
- **When a change is a job:** more than 1,000 issues (`BULK_JOB_ABOVE`); an access key may not start one (`auth.access_key_not_allowed`, M3-F121).
- **Jobs that come back every day.** `editing.purge-drafts` (drafts, notes for people who were editing and lock rows that ran out) and `history.catch-up` (the history reads the events, ADR-0007) are daily kinds; there is no timer of the server any more (M3-F30, M3-F71). `bb-server` calls `Ctx::start_daily_jobs` when it starts, which queues a round of each at once, and every round queues the next one a day later **before** it does its work, so a round that fails (the queue tries it again) or a worker that stops does not end the days. A round queues the next only when none for a later time waits (`JobStore::later_job_waits`), so a server that starts twice in a day leaves one chain of rounds, not two. Both are idempotent, and a read of a history still catches up first.
- **Permissions are cached for one step.** A step of `bulk.step` judges its (up to 100) issues with one context, which keeps what the policy engine decided for the person for the whole step (ADR-0025, decision 3: the cache lives as long as its context). A person who loses access while a run goes on is therefore stopped at the next step, not at the next issue: the next step builds a new context and judges from the roles they hold then, and the issues it leaves alone are named with `not-found` or `not-allowed`.
- **A stopped worker and the count.** The change to an issue and the saving of the progress of the run are two writes. The event of every change a run makes carries a reason that names the run and the item (`BulkRun::item_reason`); a step that begins at an item whose event is there already (`BulkStore::event_with_reason`, looked up by the range of event IDs since the run began) counts the issue as changed and goes on, instead of judging it again and reporting that it changed after the preview.
- **A tag or label counts the version.** Putting a tag or a label on an issue or taking one off counts the version of the issue up in the same change (by one, whatever the version stored then is: the routes do not take a version, so two people who add or take off different ones both succeed, and so does a tag put on while somebody saves the title; the event carries the version the row has after the count-up. The other way round, a save that read the version before the tag was put on is told that the version changed, like after any other change. An issue that went to the trash in between is not found), so a preview that read the version before sees that the issue changed in between and names it, also in a *Pick one* group where the label it would replace is not the one the issue has now.
- **One project event for the renewal.** `ChangeStore::record_changes` writes a change with any number of events of its own, also none. A step of `issues.renew-ids` writes the events of the issues it renews; the step that renews the last also writes the one event of the project (`issue-ids`).
