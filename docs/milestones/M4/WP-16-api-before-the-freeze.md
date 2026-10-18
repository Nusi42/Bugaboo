# WP-16 — The API before the freeze

**Exit wave B · Folder / branch:** `bugaboo.win2` / `WIN2`, after WP-12 to WP-15 are merged
**Features:** WF-01…WF-06, ATT-01…ATT-07, NTF-01, NTF-02 (their API).
**Source:** [the M4 exit review](exit-review.md), architecture section *API changes to make before the freeze* and check 5; ADR-0008 amendment M4; follow-ups M4-F53, M4-F65.

## Goal

The API that is frozen at `m4-done` is consistent, small and in the glossary's words, and ARCH-API-FREEZE keeps it that way.

## Decided by the lead

- **A1:** kebab-case for every enum value and every error code (`auth.wrong-credentials`), the `words!` convention. The message keys already are.
- **A5:** `readable_id`, never `key`, for an issue's ID in the API.
- **B4:** the capped previews and lists stay capped (their limits are named constants); cursors for them are an addition after the freeze.
- Everything else of A1–A10 and B1–B12 as the review proposes.

## Tasks

1. A1–A10 (must), then B1–B12 (should), in small commits, the browser and the tests following each change. M4-F65 is A2; M4-F53 is B1.
2. ARCH-API-FREEZE implemented: the frozen compact `openapi.json` is stored at `m4-done`; after that a change may only add.
3. An API lint (check 5): one casing for enum values, no duplicate or orphan schemas, no `_id` path parameters, glossary words in schema and property names (TXT-TERMS over `openapi.json`).
4. The 429/500/503 descriptions in English, not message keys.

## Depends on

WP-12 to WP-15 merged (they change routes and bodies too).
