# ADR-0021: Issue IDs, counting or random

Status: **Accepted** · 2026-10-04

## Context

Counting IDs (`WEB-142`) are easy to say aloud, but they reveal how many issues a project has and how fast it grows. Anyone who sees two IDs a week apart can estimate activity. That can be sensitive business information, and for small teams it can say something about individual people.

## Decision

- Every issue keeps its internal UUIDv7. The **readable ID** is the project's short name plus a code chosen per project:
  - **Counting:** `WEB-142`.
  - **Random:** `WEB-K7Q2X` — 5 characters (configurable 4–8) from Crockford's Base32 alphabet (no I, L, O, U, so no look-alike letters), drawn from a secure random source; on a collision, draw again.
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

## Amendment (M3)

- **Moving an issue to another project** gives it an ID of the new project (in that project's mode) and keeps the old ID in `issue_alias`, like a change of mode. M3 moves an issue only into a project that has its issue type and own fields; the preview names the issues it skips and why (decision M3-Q4). Enforced by: TEST-ISSUE-IDS.

## Amendment (M3, WP-03)

- **The mode is the setting `issues.id-mode`** (*Random IDs* or *Counting IDs*, project level and organization level, changed by the project's lead). A sub-project follows its parent until it chooses, like every setting in layers (ADR-0018). The length of a random code is the setting `issues.id-length` (4 to 8 symbols, 5 to start, organization level); when the short codes of a project run out, the code gets one symbol longer by itself.
- **Changing the mode of a project** (setting or resetting it) gives every issue of the project, and of the sub-projects that follow it, a code of the new mode in the same change, and keeps each old code in `issue_alias` with its project. A key is found by the short name of the project (now or an old one) and the code (now or an old one); the answer holds the current ID. A code that was given out is never given out again in its project, not even to another issue: the stored counter only goes up, and a random draw also avoids the old codes. **The organization's default does not renew anything:** an issue that was made keeps its ID, and projects that follow the default give their next issues the new mode (M3-F20).
- **The counting number is the one counter of the product** (`issue_counter`, one row per project, raised inside the change that makes the issue, so two writers wait for each other and no number is skipped). ARCH-NO-COUNTERS names the files that may use it. Random codes come from the secure random source; a clash with a code of the project is drawn again.
- Enforced by: TEST-ISSUE-IDS (the tests of these rules exist by name and the database ones run on both databases), ARCH-NO-COUNTERS.

## Amendment (M1)

- Records keep a UUIDv7 (time-ordered). A person's ID is a fully random UUIDv4 instead, so it does not show when the account was made.
- `bb-core` does no I/O and builds for WebAssembly, so it never reads the clock or the system's random source. An ID is made from a time and random bytes that the caller passes in (`from_parts`, `IdSource`); the server's context supplies the real ones (WP-05).
- Checked by ARCH-WASM (`bb-core` builds for `wasm32-unknown-unknown`) and the tests `person_ids_are_random_version_4` and `from_parts_puts_the_time_into_the_id` in `crates/bb-core/src/id.rs`.
