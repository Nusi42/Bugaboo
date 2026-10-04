# WP-02 — Browser toolchain, tokens and texts

**Folder / branch:** `<work root>/bugaboo.win2` / `WIN2`
**Features:** groundwork for UX-07 (message catalog), THM-01/02 (themes), PERF-04 (bundle budget later), ADR-0009, ADR-0010, ADR-0023.

## Goal
A browser app skeleton that already obeys every frontend rule: TypeScript 7, React 19, Vite 8, formatted by oxfmt, linted by oxlint with type-aware tsgolint, CSS checked by stylelint, unit-tested with Vitest, end-to-end tested with Playwright, all on the ports from `tools/dev/ports.json`. Colors come only from generated design tokens; text only from the message catalog.

## Owns
`package.json`, `package-lock.json` (npm workspaces), `web/**`, `design/tokens.json`, `messages/**`, `docs/design/tokens.css` (becomes generated), frontend tool configs at the root (oxlint, oxfmt, stylelint), `xtask/src/gen/**`, and these checks in `xtask/src/checks/`: `fmt_ts.rs`, `fmt_css.rs`, `lint_ts.rs`, `lint_types.rs`, `lint_css.rs`, `dev_ports.rs`, `sty_contrast.rs`, `txt_catalog.rs`.

## Tasks
1. **npm workspaces** at the root: `web/app`, `web/packages/ui`, `web/packages/messages`. Exact pinned versions (architecture §9): react/react-dom 19.3.0, typescript 7.0.2, vite 8.3.2, vitest 5.0.3, @playwright/test 1.63.0, @axe-core/playwright 4.13.0, oxlint 1.86.0, oxlint-tsgolint 7.0.2003, oxfmt 0.71.0, stylelint 17.16.0. No `^` or `~` ranges.
2. **TypeScript:** `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `noImplicitOverride`, `noFallthroughCasesInSwitch`, `verbatimModuleSyntax`. One shared base `tsconfig`.
3. **oxlint** with type-aware rules on (tsgolint), categories correctness, suspicious, perf and pedantic as errors; rules banning `console`, `any`, non-null `!`, `@ts-ignore`, `dangerouslySetInnerHTML`, `eval`; react and react-hooks plugins. **oxfmt** for TS/TSX/JS/JSON/CSS (if oxfmt can't format CSS, use Biome for CSS only and record that in the register). **stylelint** for CSS Modules: no hex/rgb/hsl colors, no named colors, no `!important`, lengths only via `var(--…)` tokens (except 0 and 1px borders).
4. **Design tokens:** create `design/tokens.json` (W3C Design Tokens format) from the values in `docs/design/tokens.css` (four themes). `cargo xtask gen tokens` writes `web/packages/ui/src/tokens.css` and `tokens.ts`, and regenerates `docs/design/tokens.css` (header: generated, don't edit). The theme picker logic (Match my device → Daylight/Dusk) lives once, in `web/packages/ui`.
5. **Messages:** `messages/en.json` (ICU message format). `cargo xtask gen messages` writes typed keys to `web/packages/messages`. The app shows text only through these keys.
6. **App skeleton** (`web/app`): Vite config reads `tools/dev/ports.json` (`web`, `webPreview`) with `strictPort: true`, and `cacheDir` inside the work folder. A "Hello Bugaboo" screen with the brand, the theme picker (Match my device, Daylight, Paper, Dusk, Midnight) and one sentence, all from tokens and messages. Accessible: labels, focus ring, works at 360 px width.
7. **Tests:** one Vitest unit test (theme resolution), one Playwright test that opens the preview build on `webPreview` and checks the page in all four themes with axe (0 violations) at desktop and phone size.
8. **Checks** (catalogue entries set to active, each with bad and good examples): FMT-TS, FMT-CSS, LINT-TS, LINT-TYPES, LINT-CSS, DEV-PORTS, STY-CONTRAST (WCAG AA for every text/background pair in the four built-in themes — adjust token values until they pass; record changes in the PR description), TXT-CATALOG.
9. **Register** every new npm package in `docs/third-party/README.md`.

## Acceptance
- `npm ci` then `npm run build` and `npm test` (Vitest) pass; `npx playwright test` passes on 47202.
- `cargo xtask check only FMT-TS FMT-CSS LINT-TS LINT-TYPES LINT-CSS DEV-PORTS STY-CONTRAST TXT-CATALOG` passes; each check's bad example fails.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` stay at 0 warnings / 0 failures.
- No default ports anywhere; no raw colors or literal UI text in `web/`.
