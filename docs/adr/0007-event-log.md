# ADR-0007: Event log and outbox as the single source of change

Status: **Accepted** · 2026-10-04

## Context

History, notifications, webhooks, the security log, full-text indexing and live updates all need to know what changed. Writing to each separately would duplicate logic and lose consistency.

## Decision

Every use case appends **domain events** (actor, channel, entity, field changes before → after, reason) in the **same transaction** as the change, plus an **outbox** row. After commit, consumers (history, notifications, webhooks, security log, search index, live updates) read the outbox, remember their position, and are idempotent.

## Consequences

- One write path; every consumer sees the same facts.
- The security log is hash-chained from the security-relevant events.
- Events contain personal data, so they carry personal-data marks and follow keep-and-delete rules (ADR-0015).

## Change (M3 WP-07)

The first consumer of the outbox is the history of an issue (`history_entry`, ISS-06). It reads events after its stored position and writes one row per event and issue with `ON CONFLICT DO NOTHING`, so a repeat changes nothing. The rows commit first and the position moves after them in a second step, so a stop in between makes the next round read the same events again and the repeat is harmless; it resumes after a restart. The rows hold the facts; the reader's field mask is applied when the history is read, never when it is built, so a change of roles changes what is shown at once. The consumer runs when a history is read and as the daily job `history.catch-up` of the job runner (ADR-0017), which is also queued at every server start (M3-F71).

## Alternatives considered

- **Full event sourcing:** more power than needed and harder for agents to get right. Rejected; current state stays in normal tables.
- **Database triggers:** dialect-specific and invisible in code. Rejected.

## Enforced by

ARCH-EVENTS (see `tools/checks/catalog.toml`).
