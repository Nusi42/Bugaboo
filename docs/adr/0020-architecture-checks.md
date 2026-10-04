# ADR-0020: Architecture is checked by many small checks, and the checks are checked

Status: **Accepted** · 2026-10-04

## Context

AI agents implement Bugaboo. Written rules drift unless machines enforce them. One big "lint everything" step is slow, hard to understand and tempting to skip.

## Decision

- **One catalogue** (`tools/checks/catalog.toml`) lists every check: ID, plain-language title, the rule it enforces, the tool, when it runs (edit · commit · push · CI · nightly · release) and which files trigger it. `cargo xtask` implements them; lefthook, Claude Code hooks and CI only call xtask.
- **Many small checks** instead of one big one: each enforces one rule, runs only when its files change, and explains itself in one sentence with a link to the rule.
- **Checks for the checks (META):**
  - every catalogue entry is implemented, and nothing is implemented without an entry;
  - every check has a failing and a passing example;
  - every file type in the repository is covered by a formatter or linter, so a new kind of file forces a new check;
  - a new crate, package or folder needs a module-map entry first;
  - every "must" rule in the docs names its check or says "review" with a reason (META-RULE-TRACE);
  - all three callers use the same lists;
  - suppressions need a reason and an issue ID, and may not quietly grow (META-SUPPRESS);
  - edit- and commit-time checks stay fast.
- **Agents** get edit-time checks after every file change, so mistakes are fixed while the context is fresh.

## Consequences

- Adding a rule means: write it in the docs, add a catalogue entry, implement it in xtask, and add the two examples. The meta checks fail until all four are done.
- The architecture documentation stays true, because drift breaks the build.

## Alternatives considered

- **Review only:** doesn't scale with agents. Rejected.
- **Separate hook scripts per tool:** differ between Windows and Linux and drift from CI. Rejected.

## Enforced by

META-CATALOG, META-FIXTURES, META-COVERAGE, META-NEW-MODULE, META-RULE-TRACE, META-WIRING, META-SUPPRESS, META-SPEED (see `tools/checks/catalog.toml`).
