# WP-04 — Docs builder and meta checks

**Folder / branch:** `<work root>/bugaboo.win4` / `WIN4`
**Features:** COMP-06 (architecture docs stay true), PLAN-TRACE; ADR-0020.

## Goal
The commentable HTML pages are built by `cargo xtask docs` (Rust), and the meta checks guard the catalogue, the docs and the checks themselves.

## Owns
`xtask/src/docs/**`, and these checks in `xtask/src/checks/`: `doc_generated.rs`, `doc_adr_enforced.rs`, `plan_trace.rs`, `arch_map_sync.rs`, `meta_fixtures.rs`, `meta_coverage.rs`, `meta_new_module.rs`, `meta_rule_trace.rs`, `meta_wiring.rs`, `meta_suppress.rs`, `meta_speed.rs`. You remove `tools/docs-html/` (and its register row) once the Rust builder produces the same pages.

## Tasks
1. **`cargo xtask docs`**: port `tools/docs-html/build.mjs` to Rust using `pulldown-cmark` (latest stable). Output must match the current pages in structure: `docs/03-architecture.html` (chapters, numbered sections, decision records, the check table from the catalogue) and `docs/04-milestones.html`. Mermaid code blocks become `<pre class="mermaid">`. Links between Markdown files become in-page anchors. Generated files start with a "generated, don't edit" comment.
2. **Checks** (catalogue entries set to active, each with bad and good examples):
   - DOC-GENERATED: regenerating docs (and, once they exist, tokens and messages from WP-02) produces no diff.
   - DOC-ADR-ENFORCED: every ADR has an `## Enforced by` section naming catalogue IDs (or `review` with a reason); every named ID exists.
   - PLAN-TRACE: every M and V1 feature ID in `docs/01-feature-list.md` appears in exactly one milestone of `docs/04-milestones.md`; M features only in M0–M9; every work package file in `docs/milestones/**` names feature IDs that exist.
   - ARCH-MAP-SYNC: the crate list in `docs/architecture/04-building-blocks.md` (repository layout) equals the workspace members, and the browser packages listed there equal the npm workspaces (once they exist).
   - META-FIXTURES: every active check has a `bad_example` and a `good_example` test or a `tools/checks/fixtures/<ID>/bad|good/` folder.
   - META-COVERAGE: every tracked file type (by extension) is matched by at least one active FMT-* or LINT-* check's `files` patterns; list the uncovered types.
   - META-NEW-MODULE: every crate and npm package is listed in the module map (shares the parser with ARCH-MAP-SYNC — one implementation).
   - META-RULE-TRACE: every line in `docs/coding-standards.md` and `docs/adr/*.md` containing "must" names a catalogue ID or "review" in the same paragraph; start with the ADRs and coding standards, report the rest as findings to fix.
   - META-WIRING: `lefthook.yml`, `.claude/settings.json` and `.github/workflows/*.yml` call only `cargo xtask …` (passes while those files don't exist yet; WP-05 adds them).
   - META-SUPPRESS: every `#[allow(` / `#![allow(`, `oxlint-disable`, `stylelint-disable`, `ast-grep-ignore` has a reason (`reason = "…"` or a trailing comment) naming an issue or doc section; the total is written to `tools/checks/suppressions.txt` and may only grow when that file is updated in the same change.
   - META-SPEED: `check edit` and `check staged` record each check's time; fail if an edit-time check takes over 3 s or a commit run over 10 s (measured on the current machine; report numbers).
3. **Register** `pulldown-cmark` (and anything else new); remove the `marked` row with `tools/docs-html`.

## Acceptance
- `cargo xtask docs` regenerates both pages; opening them shows the same content as today's published pages.
- `cargo xtask check all` passes with these checks active; each bad example fails.
- `cargo fmt --check`, Clippy with `-D warnings`, `cargo test --workspace`: 0 warnings, 0 failures.
