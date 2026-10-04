# Tech stack

Latest stable versions, checked against crates.io and npm on **2026-10-04**. Lockfiles pin exact versions; updates are deliberate (Renovate-style pull requests, reviewed). Every direct dependency also needs an entry in `docs/third-party/` (coding-standards §5).

## Toolchain

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.97.1 | edition 2024; `rust-toolchain.toml` pins it |
| Node.js | 26.x | for building the browser app and tooling only; not needed to run Bugaboo |
| npm | 11.x | workspaces for `web/` |
| TypeScript | 7.0.2 | native compiler |

## Server (Rust crates)

| Area | Crate | Version | License |
|------|-------|---------|---------|
| Async runtime | tokio | 1.53.2 | MIT |
| HTTP | axum | 0.8.9 | MIT |
| HTTP middleware | tower-http | 0.7.1 | MIT |
| Rate limits | tower_governor | 0.8.0 | MIT/Apache-2.0 |
| TLS | rustls / tokio-rustls | 0.23.45 / 0.26.6 | Apache-2.0/MIT/ISC |
| Certificates (ACME) | instant-acme | 0.8.5 | Apache-2.0 |
| Database access | sqlx | 0.9.0 | MIT/Apache-2.0 |
| Query builder | sea-query + sea-query-sqlx | 1.0.2 + 0.9.1 | MIT/Apache-2.0 |
| OpenAPI | utoipa | 6.0.0 | MIT/Apache-2.0 |
| Serialization | serde | 1.0.229 | MIT/Apache-2.0 |
| Config file | toml | 1.1.6 | MIT/Apache-2.0 |
| CLI | clap | 4.6.7 | MIT/Apache-2.0 |
| Windows service | windows-service | 0.8.1 | MIT/Apache-2.0 |
| Passwords | argon2 | 0.6.0 | MIT/Apache-2.0 |
| Passkeys | webauthn-rs | 0.5.5 | MPL-2.0 (allowed, unmodified) |
| Two-step codes | totp-rs | 6.0.0 | MIT |
| Company sign-in | openidconnect | 4.0.1 | MIT |
| Tokens (JWT) | jsonwebtoken | 11.1.0 | MIT |
| Encryption | chacha20poly1305 | 0.11.0 | MIT/Apache-2.0 |
| Hashing / MAC | sha2 / hmac | 0.11.0 / 0.13.0 | MIT/Apache-2.0 |
| Random | rand | 0.10.3 | MIT/Apache-2.0 |
| IDs | uuid (v7) | 1.27.0 | MIT/Apache-2.0 |
| Dates and time zones | jiff | 0.2.37 | MIT/Unlicense |
| Errors | thiserror | 2.0.21 | MIT/Apache-2.0 |
| Logging | tracing | 0.1.44 | MIT |
| Markdown | pulldown-cmark | 0.13.4 | MIT |
| HTML sanitizing | ammonia | 4.2.1 | MIT/Apache-2.0 |
| Embedded web app | rust-embed | 8.12.0 | MIT |
| File type detection | infer | 0.22.0 | MIT |
| Image previews | image | 0.25.10 | MIT/Apache-2.0 |
| PDF previews | hayro | 0.7.1 | to check in the register |
| File storage (S3) | object_store | 0.14.2 | MIT/Apache-2.0 |
| E-mail | lettre | 0.11.23 | MIT |
| MCP | rmcp | 3.5.0 | MIT/Apache-2.0 |
| WebAssembly | wasm-bindgen | 0.2.129 | MIT/Apache-2.0 |
| Tests | proptest / insta | 1.11.0 / 1.49.0 | MIT/Apache-2.0 |

Exact license strings are confirmed by `cargo deny` and recorded in the third-party register; this table is a planning overview.

## Browser app (npm packages)

| Area | Package | Version | License |
|------|---------|---------|---------|
| UI library | react / react-dom | 19.3.0 | MIT |
| Build | vite | 8.3.2 | MIT |
| Routing | @tanstack/react-router | 1.170.41 | MIT |
| Server data | @tanstack/react-query | 5.104.1 | MIT |
| Long lists | @tanstack/react-virtual | 3.14.13 | MIT |
| Accessible components | react-aria-components | 1.21.1 | Apache-2.0 |
| Document editor | @tiptap/react (ProseMirror) | 3.31.4 | MIT |
| Relation map layout | d3-force | 3.0.0 | ISC |
| Message formatting | intl-messageformat | 12.1.2 | BSD-3-Clause |
| API types | openapi-typescript | 7.13.0 | MIT |

## Quality and compliance tooling

| Purpose | Tool | Version | License |
|---------|------|---------|---------|
| TS/JS/JSON format | oxfmt | 0.71.0 | MIT |
| TS lint | oxlint | 1.86.0 | MIT |
| TS type-aware lint (TypeScript 7) | oxlint-tsgolint | 7.0.2003 | MIT |
| CSS lint (tokens only) | stylelint | 17.16.0 | MIT |
| Fallback formatter | @biomejs/biome | 2.5.15 | MIT/Apache-2.0 |
| Pattern rules (Rust + TS) | @ast-grep/cli | 0.45.3 | MIT |
| Unit tests (browser code) | vitest | 5.0.3 | MIT |
| Local ports | `tools/dev/ports.json` | — | — (47200–47299, strict; ADR-0023) |
| End-to-end tests | @playwright/test | 1.63.0 | Apache-2.0 |
| Accessibility checks | @axe-core/playwright | 4.13.0 | MPL-2.0 (test only, not shipped) |
| Git hooks | lefthook | 2.1.16 | MIT |
| Rust format / lint | rustfmt / clippy | with Rust 1.97.1 | MIT/Apache-2.0 |
| TOML format | taplo-cli | 0.10.0 | MIT |
| Markdown lint | rumdl | 0.2.78 | MIT |
| Spelling | typos-cli | 1.50.3 | MIT/Apache-2.0 |
| Link check | lychee | 0.24.2 | MIT/Apache-2.0 |
| Unused Rust dependencies | cargo-machete | 0.9.2 | MIT/Apache-2.0 |
| Secret scan | gitleaks | latest at setup | MIT |
| CI file lint | actionlint | latest at setup | MIT |
| Licenses, bans, advisories | cargo-deny | 0.20.2 | MIT/Apache-2.0 |
| Third-party notices | cargo-about | 0.9.2 | MIT/Apache-2.0 |
| SBOM | cargo-cyclonedx / @cyclonedx/cyclonedx-npm | 0.5.9 / 6.0.1 | Apache-2.0 |
| Docs → HTML (temporary) | marked | 18.0.14 | MIT |

## Considered and not chosen

| Option | Why not |
|--------|---------|
| tantivy (search engine) | A second index outside the database makes permission filtering harder; database full-text search is enough for the scale target (ADR-0016) |
| SeaORM | We need explicit SQL control for permission predicates; sea-query + sqlx gives that without an ORM layer |
| ESLint + typescript-eslint | typescript-eslint supports TypeScript only up to 6.0; oxlint with tsgolint is built on TypeScript 7 itself |
| Biome as main linter | Its type-aware rules use its own inference, not the TypeScript 7 compiler; kept as fallback formatter |
| Tailwind CSS | Utility classes make the "only tokens" rule harder to check; CSS Modules with token variables are simpler to enforce |
| Commercial Gantt/scheduler libraries | License cost and terms conflict with both distribution models; ours is built on shared primitives |
| elkjs (graph layout) | EPL-2.0 license is not on the allow list; d3-force (ISC) is enough for the relation map |
