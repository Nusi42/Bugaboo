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
