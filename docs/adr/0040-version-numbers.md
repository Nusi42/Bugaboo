# ADR-0040: Version numbers follow the plan

Status: **Accepted** · 2026-10-31

## Context

Bugaboo is published before it is finished. Its version number should tell a reader how far it is, and it should not be chosen by feeling. The plan already measures progress: [04-milestones](../04-milestones.md) lists every milestone with the number of features it brings, and a finished milestone gets a tag `mN-done`.

## Decision

1. **0.9.0 means feature complete:** every feature of the plan is built, that is every milestone up to M14. M15 (live updates and the release itself) brings **1.0.0**.
2. **Before that, the minor number follows the features done:** `minor = floor(9 × done / total)`, where `done` counts the features of the milestones that carry their `mN-done` tag, and `total` the features of M0 to M14 (the *Features* column of the overview in 04-milestones). With M0 to M5 done that is `floor(9 × 96 / 245) = 3`: **0.3.0**.
3. **The patch number** counts the releases made at the same minor number, from 0. A finished milestone that does not raise the minor number raises the patch number.
4. The version is written once, in `[workspace.package]` of `Cargo.toml`, which every crate inherits; the browser packages carry no version of their own. It changes in the commit that records a finished milestone.

## Consequences

- The number moves only when a milestone is done, so it never claims work that is not tagged.
- A change of the plan's feature counts changes the arithmetic; the version is not lowered for that, the next milestone simply catches up.

## Alternatives considered

- **The milestone number as minor (M5 → 0.5.0):** the plan has more milestones than nine before 1.0, so the numbers would run out. Rejected.
- **Calendar versions:** say nothing about how complete Bugaboo is. Rejected.

## Enforced by

review (the arithmetic needs the plan's table and the milestone tags, which no check reads yet): checked at the commit that records a finished milestone.
