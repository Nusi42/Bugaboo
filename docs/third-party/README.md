# Third-party register

Every **direct** dependency of Bugaboo — shipped or used only for building, testing or docs — has one row here, added in the same change that adds the dependency (coding-standards §5). The register check (`cargo xtask check only DEP-REGISTER`, coding-standards §7) compares these tables with `Cargo.lock` and the npm lockfiles and fails on any difference.

Together with the lockfiles, this register is the source for the SBOM, the third-party notices and *About › Licenses* (COMP-01, COMP-02).

Columns:

- **Ships?** — *yes* if it ends up in the Bugaboo binary or browser app; *no* for build, test and docs tools.
- **Health** — maintained / slow / unmaintained, plus the date checked.
- **Alternatives** — what else was considered and why not.

## Tooling (npm)

| Package | Version | License | Ships? | Purpose | Health | Alternatives |
|---------|---------|---------|--------|---------|--------|--------------|

## Rust

| Crate | Version | License | Ships? | Purpose | Health | Alternatives |
|-------|---------|---------|--------|---------|--------|--------------|
| anyhow | 1.0.104 | MIT OR Apache-2.0 | yes (bb-server), no (xtask) | Error handling with context in the check runner | maintained (2026-10-04) | thiserror (better for libraries; the runner only reports errors) |
| clap | 4.6.7 | MIT OR Apache-2.0 | yes (bb-server), no (xtask) | Command line of `cargo xtask` | maintained (2026-10-04) | argh, pico-args (less help text, fewer checks) |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | no (xtask) | Unpacking the `.tar.gz` archives of downloaded tools in `cargo xtask setup` | maintained (2026-10-04) | system `tar` (differs between Windows and Linux) |
| globset | 0.4.20 | Unlicense OR MIT | no (xtask) | Matching changed files against the catalogue's `files` patterns | maintained (2026-10-04) | glob (no sets, no `{a,b}`), wildmatch |
| pulldown-cmark | 0.13.4 | MIT | no (xtask) | Turns the Markdown docs into the commentable HTML pages (`cargo xtask docs`); built without its command-line feature | maintained (2026-10-04) | comrak (heavier, many extensions we don't need); markdown-rs (less used); bb-markdown (the product renderer; xtask must not depend on product crates) |
| serde | 1.0.229 | MIT OR Apache-2.0 | no (xtask) | Reading the check catalogue and tool list | maintained (2026-10-04) | hand-written parsing (error-prone) |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | no (xtask) | Reading `cargo metadata`, `package.json` / `package-lock.json`, design/tokens.json, tools/dev/ports.json, the message catalog and the Claude Code hook input; `preserve_order` keeps file order in generated output | maintained (2026-10-04) | hand-written parsing (error-prone) |
| sha2 | 0.11.0 | MIT OR Apache-2.0 | no (xtask) | Verifying the sha256 of every downloaded tool before it is unpacked | maintained (2026-10-04) | ring (large, C and assembly code); calling `certutil` / `sha256sum` (differs per system) |
| tar | 0.4.46 | MIT OR Apache-2.0 | no (xtask) | Reading the `.tar.gz` archives of downloaded tools | maintained (2026-10-04) | system `tar` (differs between Windows and Linux) |
| toml | 1.1.6 | MIT OR Apache-2.0 | no (xtask) | TOML parser for the catalogue and tool list | maintained (2026-10-04) | toml_edit (only needed for writing) |
| zip | 8.6.0 | MIT | no (xtask) | Reading the `.zip` archives of downloaded tools on Windows | maintained (2026-10-04) | PowerShell `Expand-Archive` (not on Linux) |
| ctrlc | 3.5.2 | MIT OR Apache-2.0 | no (xtask) | Stops the dev servers cleanly on Ctrl+C or termination (`cargo xtask dev`) | maintained (2026-10-04) | own signal handling (needs unsafe or platform code) |
| axum | 0.8.9 | MIT | yes | HTTP routing and server (ADR-0002) | maintained (2026-10-04) | actix-web (smaller tower middleware ecosystem) |
| mime_guess | 2.0.5 | MIT | yes | Content type from a file name when serving the browser app | maintained (2026-10-04) | own table (would be a copy of knowledge that exists) |
| rust-embed | 8.12.0 | MIT | yes | Embeds the built browser app in the binary (DIST-01); reads from disk in debug builds | maintained (2026-10-04) | include_dir (no debug-from-disk mode), build script with include_bytes (more code) |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | no (tests) | Temporary folders in tests | maintained (2026-10-04) | hand-made folders in the system temp folder (leaks on failure) |
| tokio | 1.53.2 | MIT | yes | Async runtime of the server (ADR-0002) | maintained (2026-10-04) | async-std (discontinued) |
| tower | 0.5.3 | MIT | no (tests) | `oneshot` to call the router in tests without a socket | maintained (2026-10-04) | starting a real server per test (slower, needs ports) |
| tower-http | 0.7.1 | MIT | yes | Sets the security headers on every response (SEC-11) | maintained (2026-10-04) | own middleware (more code, same result) |

Server crates are added with the milestones that use them; planned ones are listed in [architecture/09-tech-stack.md](../architecture/09-tech-stack.md).

## Browser app (npm)

| Package | Version | License | Ships? | Purpose | Health | Alternatives |
|---------|---------|---------|--------|---------|--------|--------------|
| react | 19.3.0 | MIT | yes | UI library (ADR-0009) | maintained (2026-10-04) | Preact (smaller, but React Aria and Tiptap target React) |
| react-dom | 19.3.0 | MIT | yes | Renders React in the browser | maintained (2026-10-04) | — |
| intl-messageformat | 12.1.2 | BSD-3-Clause | yes | Formats ICU messages from the message catalog (UX-07) | maintained (2026-10-04) | @formatjs/intl (larger, adds React bindings we do not need); own parser (error-prone) |
| vite | 8.3.2 | MIT | no | Builds the browser app and runs the dev server (ADR-0009, ADR-0023) | maintained (2026-10-04) | webpack, Rspack (slower, more configuration) |
| typescript | 7.0.2 | Apache-2.0 | no | Type checker (`tsc`, native compiler; ADR-0014) | maintained (2026-10-04) | TypeScript 6 (typescript-eslint stack, rejected in ADR-0014) |
| vitest | 5.0.3 | MIT | no | Unit tests for browser code (ADR-0023) | maintained (2026-10-04) | Jest (needs extra transforms for Vite projects) |
| @playwright/test | 1.63.0 | Apache-2.0 | no | End-to-end tests in real browsers (ADR-0023) | maintained (2026-10-04) | Cypress (no multi-context concurrency tests, heavier) |
| @axe-core/playwright | 4.13.0 | MPL-2.0 | no | Accessibility checks in end-to-end tests (test only, not shipped) | maintained (2026-10-04) | pa11y (less integrated with Playwright) |
| oxlint | 1.86.0 | MIT | no | TypeScript and React linter (ADR-0014) | maintained (2026-10-04) | ESLint + typescript-eslint (supports TypeScript only up to 6.0) |
| oxlint-tsgolint | 7.0.2003 | MIT | no | Type-aware rules for oxlint, built on the TypeScript 7 compiler | maintained (2026-10-04) | typescript-eslint type-aware rules (TypeScript 6 only) |
| oxfmt | 0.71.0 | MIT | no | Formats TypeScript, JSON and CSS (ADR-0014); also covers CSS, so Biome is not needed | maintained (2026-10-04) | Biome (kept as fallback, not installed); Prettier (slower) |
| stylelint | 17.16.0 | MIT | no | Keeps CSS Modules to design tokens (LINT-CSS, ADR-0010) | maintained (2026-10-04) | Biome CSS lint (no rule for banning raw sizes by property) |
| @vitejs/plugin-react | 6.1.1 | MIT | no | React support for Vite 8 (Oxc-based JSX transform and fast refresh) | maintained (2026-10-04) | none needed before; Vite alone compiles JSX but gives no fast refresh |
| html-validate | 11.16.2 | MIT | no | Validates the hand-written HTML pages (LINT-HTML)  | maintained (2026-10-04) | Nu HTML Checker (needs Java); htmlhint (fewer accessibility rules) |
| @types/node | 26.6.4 | MIT | no | Node types for the Vite, Vitest and Playwright configs | maintained (2026-10-04) | — |
| @types/react | 19.3.0 | MIT | no | React types | maintained (2026-10-04) | — |
| @types/react-dom | 19.3.0 | MIT | no | React DOM types | maintained (2026-10-04) | — |

## Accepted risks

| Advisory | Package | Severity | Why accepted | Re-check |
|----------|---------|----------|--------------|----------|
| [GHSA-vfj7-8cjw-p6xm](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm) | braces (3.0.3, via stylelint, globby, fast-glob, micromatch) | high (stack exhaustion through deeply nested patterns) | No patched release exists: the advisory covers every version, and the only suggested fix downgrades stylelint to 7.7.0. The package is a dev-only tool (stylelint globbing our own files); it is not shipped and never receives untrusted patterns. | 2027-01-04, or when braces, micromatch or stylelint publish a new release |
