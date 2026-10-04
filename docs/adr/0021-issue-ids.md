# ADR-0021: Issue IDs, counting or random

Status: **Accepted** · 2026-10-04

## Context
Counting IDs (`WEB-142`) are easy to say aloud, but they reveal how many issues a project has and how fast it grows. Anyone who sees two IDs a week apart can estimate activity. That can be sensitive business information, and for small teams it can say something about individual people.

## Decision
- Every issue keeps its internal UUIDv7. The **readable ID** is the project's short name plus a code chosen per project:
  - **Counting:** `WEB-142`.
  - **Random:** `WEB-K7Q2X` — 5 characters (configurable 4–8) from Crockford's Base32 alphabet (no I, L, O, U, so no look-alikes), drawn from a secure random source; on a collision, draw again.
- The **organization default starts as random** (security first); each project can choose.
- **Changing the mode** or **moving an issue** to another project gives it a new readable ID; the old ID keeps redirecting (`issue_alias` table), so links never break.
- Search, links in text (`WEB-K7Q2X` becomes a chip) and the API accept both kinds.
- No other public identifier counts up either: API resources use readable IDs or UUIDs, removed people get random labels, and no screen shows totals that include hidden items.

## Consequences
- The issue chip parser (bb-markdown) recognizes `SHORT-CODE` for both forms.
- People in random-ID projects can't guess neighboring issues, which also makes probing for hidden issues harder.

## Alternatives considered
- **Counting only:** leaks volume. Rejected as the only option.
- **UUIDs in the UI:** too long to read or say. Rejected.

## Enforced by
TEST-ISSUE-IDS, ARCH-NO-COUNTERS (see `tools/checks/catalog.toml`).
