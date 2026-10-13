# ADR-0017: Background jobs stored in the database

Status: **Accepted** · 2026-10-04

## Context

Previews, virus scans, e-mail, webhooks, digests, backups, keep-and-delete runs and person removal need reliable background work.

## Decision

A small job queue in the database (`job` table with `run_at`, attempts, lease) run by `bb-jobs` inside the server process. Leases use `SELECT … FOR UPDATE SKIP LOCKED` on PostgreSQL and the single writer queue on SQLite. Schedules (cron-like) are settings in the registry.

## Consequences

- Nothing extra to install; jobs survive restarts.
- Throughput is enough for the scale target; heavy work (previews) is limited by a worker pool size setting.

## Alternatives considered

- **Redis/RabbitMQ:** more installation and attack surface. Rejected.

## Enforced by

review; every job type has an idempotency test (part of ARCH-EVENTS test harness) (see `tools/checks/catalog.toml`).

## Amendment (M3 WP-08)

- **The runner exists (`bb-jobs`).** It depends on `bb-app` only (ARCH-DEPS): `spawn(services, every)` calls `Ctx::run_due_jobs()` on a timer, and `bb-server` starts it. The queue is behind the port `JobStore`; a round takes up to five due jobs under a lease of 120 seconds, runs each, reports it done (it leaves the queue) or failed, and a job that did a step queues the next one when more is left. A job that failed five starts goes to the dead-letter list.
- **Kinds so far:** `bulk.step` (changing many issues: 100 issues in a step, progress kept in the table `bulk_run` after each issue, so a step that stops half way goes on at the next issue and no issue is changed twice) and `issues.renew-ids` (ADR-0021). Both judge again at run time what they do, with the permissions of the person who started them (`Ctx::acting`), and are idempotent.
- **When a change is a job:** more than 1,000 issues (`BULK_JOB_ABOVE`); an access key may not start one (`auth.access_key_not_allowed`, M3-F121).
