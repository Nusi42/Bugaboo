# Clean-up audit

Findings of the read-only audit of all existing code against the SSOT/DRY/ODR review gate (coding-standards §1) and the single-theme-file rule. Four agents read the code on `main` at `03808a7`. Each finding names its copies and the one home it should have; the work packages in [README.md](README.md) say who fixes which. Line numbers are those of `03808a7` and move as the code changes.

## A. Rust core: bb-core, bb-policy, bb-app, bb-store, bb-auth

1. Field and people levels are written twice at every layer: bb-policy `evaluate.rs` (`FieldRule`/`PeopleRule`, `field_mask`/`people_levels`), `explain.rs`; bb-app `roles/levels.rs` (`set_field_level`/`set_people_level`), `access/visibility.rs`; bb-store `policy.rs` (four near-equal selects), `projects.rs`, the port's two `put_*_level`. Home: one level-subject trait in bb-policy.
2. The level walk exists a second time in `roles/levels.rs` (`places_above` and its loops) next to `evaluate::nearest_rule`; a missing project there becomes top level by `unwrap_or_default()`. Home: `nearest_rule` over `store.level_rules`.
3. "Which rows and rules apply at a chain" exists three times: bb-store `policy.rs` (SQL), bb-policy `tree.rs` (memory), the test fake `fake_policy.rs` (without the depth guard). `RoleRecord` to `RoleData` is copied five times. Home: `Policy::inputs_for`, `From<RoleRecord> for RoleData`.
4. "Evaluate, then narrow by the key" is written three times (`access::evaluation_of`, `tree::evaluate_as`, inline in `visible_as`); `held_in` is re-inlined twice.
5. The move rules run twice with different code (`projects/moving.rs` before and inside the change). Home: one pure function in bb-policy `moves.rs`.
6. Tree walks and the depth rule: the chain walk three times, `shown::is_inside` repeats `Tree::is_within`, `make.rs` states the depth rule differently from `too_deep_below`, `sub_projects_in` copies `subtree`, `Tree::new(projects…)` four times, `reference` equals `reference_of`.
7. `place_of` four definitions and five inline copies; two view types for the same place (`LevelOrigin`, `LevelPlace`); place-to-label three times. Home: `From<Option<ProjectId>> for Place`.
8. Paging: the (time, ID) cursor codec twice, "fetch one more and cut" five times, the keyset condition four times, batch scans twice (one with a magic `100`). Home: bb-core paging plus a store helper.
9. Mapping a failed change to an error is written four ways in different orders; catch-all handlers turn store errors into "no role label", "hidden" or an empty name.
10. Entity kinds are about 96 plain strings (`entity("role", …)`). Home: a word enum in bb-core.
11. The check "trimmed, not empty, at most N characters, no control characters" four times, with two `…_MAX = 100` constants.
12. Small copies: splitting a comma list three times, the domain check twice, hex encoding six times.
13. Knowledge held together only by tests: starting roles' level tables against the architecture tables, the permission count, error code to message key pairs, permission and starting-role keys to message keys. Home: derive the keys; generate or read the tables once.
14. The word-enum code is written by hand four or more times (`Topic`, `Counts`, `Kind`, `Permission`, `StartingRole`, `ErrorCode`). Home: export `words!` from bb-core.
15. Every store read is defined twice, for the pool and inside a change (`X` / `X_in`); `db.rs` repeats execute and fetch for pool and transaction.
16. `ENCODED_LENGTH = 43` twice, the literal `32` five times in `csrf.rs` next to `KEY_LENGTH`, about nine `Corrupt("…")` strings with inlined ID reading.
17. Bad patterns: `people_page(by_email: bool)`, settings named twice per line in `store/auth.rs`, `put_setting_value(key: &str)`, a misplaced doc line in `error_code.rs`, files far over the size guideline (`policy.rs` 1521 lines, `security_log.rs` 1285, `store/auth.rs` 1034, `ctx.rs` 775, `auth/types.rs` 771).

## B. API, server, sample, testing, xtask, tools

1. API marks typed by hand: `x-never-stored` in 33 handler attributes, `x-secret` in five; the constants exist in `openapi.rs` but are declared again in xtask and written as literals in tests; `READS_THAT_MAY_BE_KEPT` is a hand list. Home: a stored/never-stored column in `permissions::ROUTES`, the mark added by `openapi.rs`; a small shared crate that xtask may import.
2. Route knowledge in hand lists: `CSRF_EXEMPT` copied in a test (it is "public POST routes", which `ROUTES` knows); permission words repeated in five places; SEC-MATRIX actor and answer names copied; the record path twice.
3. Exemption-file handling written seven times in xtask checks (read, parse, reason rule, stale entries), with magic reason lengths. Home: `checks/exemptions.rs`.
4. A second full Rust stripper in `data_logs.rs`; line-level `starts_with("//")` filters in nine checks; the "whole word" test five times. Home: `rust_source` with a keep-strings variant and `contains_word`.
5. Three file walkers plus ad-hoc filters in twelve checks; a second `read_web_sources`; CRLF normalised inline 15 times next to `docs/source.rs::normalize`; `.tools` as a literal; the files a check covers written both in `catalog.toml` and in code. Home: one source selector driven by the catalog entry.
6. The API description is read and walked separately by seven checks and several server tests; `resolve` twice. **Bug:** the route walk in `tests/sign_in/http.rs` misses PATCH routes. Home: xtask `api_spec.rs`, a public `openapi::operations()` for tests.
7. Rules checked twice or kept in two places by a test: the 429/500/503 answers, "every route declares a permission", the `openapi.json` up-to-date test, the header table copied into `tests/common`.
8. bb-server test rigs: invite-and-accept four times, sign-in over HTTP several times, setup over HTTP 31 times, router `Options` built eight times, `type Failure` seven times, `refused()` rebuilt inline, about 200 magic numbers in `Rig::start(db, 10|50|3)`.
9. Ports: the range written in three places; a second parser of `ports.json`; `PORTS_FILE` twice.
10. `sample.json` schema defined twice (xtask and bb-sample); `DatabaseKind` three times; environment variable names copied.
11. Small ODR items: the message catalog path, the spec path, the Administrators SID twice, the Crockford alphabet three times, a never-limiting `FailureLimit` twice, CSRF header and cookie names as literals in tests, two parallel generator lists, the paged-list names repeated in `arch_lean.rs`.
12. Bad patterns: stringly typed config keys and `tls_mode`; boolean flags selecting behaviour in test helpers; five copy-paste header tests; repeated `check_files` wrappers; magic counts in asserts.

## C. Web app, UI package, e2e, tooling

1. Setting choices and defaults mirrored by hand in `shared/my-settings.ts`, `shared/region.ts`, `packages/ui/src/appearance.ts`. Home: the generator that writes `setting-keys.generated.ts` also writes the choices and defaults.
2. The cursor-paged list query written 11 times; `throw new ApiError(0, undefined)` 23 times; `(pages ?? []).flatMap(…)` 10 times. Home: `shared/paged.ts`.
3. Page sizes in 11 constants; `WHOLE_LIST = 100` restates the server maximum; hand-written field lists. Home: page sizes from `bb_core::paging` through the generator; `fields` from typed key lists.
4. Theming spread outside the token file (see section D).
5. The picker written three times (`Option`, search, error, nothing, more), identical CSS in two features. Home: one `SearchPicker` in `packages/ui`.
6. e2e duplication: about 100 page addresses typed instead of `paths.ts`, about 70 raw API strings through an untyped session, `check()` three times, `shot()` four times next to `shootWhole`, setup and sign-in and code-app setup typed by hand several times, `giveRole` bypassed, CSRF code copied, viewports and breakpoints restated, a setting key restated.
7. Project-page knowledge in four places (`settings-tabs.ts`, `use-project-page.ts`, `project-places.ts`, `paths.ts`). Home: one table.
8. `PermissionKey` defined three times; the organization-settings `anyOf` written by hand as the union of its children; pages restate place rules.
9. Person names: picking a named person four times, `PersonLine` three times, a re-implementation behind a boolean in `PeoplePreview.tsx`.
10. "Not found, else alert" gate four times; the fallback message key twice. Home: a shared `QueryGate`.
11. localStorage keys and try/catch six times. Home: one storage module with a key registry.
12. Bad patterns: boolean props selecting variants (Alert, Button, Overlay, TextField, Icon and three hooks), catch-all handlers, `shared/my-settings.ts` doing five jobs, dead `allShortcutIds`, route parameter names restated, query keys defined outside the cache module.

## D. Theming (maintainer rule)

The sister project keeps the complete look in one file: tokens with their variants per mode on one line, then `part` rules that use tokens only; components emit only `data-part`, `data-state` and `data-variant`; lint and a source test forbid `style=` and literal class names. Bugaboo today:

- 55 CSS modules with 3,261 lines (37 in the app, 18 in `packages/ui`), `global.css`, the generated `tokens.css`, and copies under `docs/design/`.
- Values without a token: `font-weight: 700` 57 times, line heights 38 times, about 50 `1px solid` borders, z-index 1/5/10/20/30/40 chosen per file, opacities, keyframes.
- Repeated rules: the focus ring 22 times although `global.css` sets `:focus-visible`, shared classes (`.stack`, `.text`, `.muted`, `.row`, `.list`) copied into up to 12 files, the skip link twice.
- High-contrast handling in ten components (`forced-colors` blocks).
- Theme names and the `data-look` attribute written by hand in TypeScript and e2e.
