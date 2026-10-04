# WP-05 — Server hello, dev runner, hooks and local CI

**Folder / branch:** `<work root>/bugaboo.win5` / `WIN5`
**Features:** DIST-01 (one binary, groundwork), groundwork for OPS-01, SEC-11; ADR-0002, ADR-0014, ADR-0023, ADR-0024.

## Goal

The `bugaboo` binary serves the embedded browser app with strict security headers and a health endpoint; `cargo xtask dev`, `build` and `ci` work; Git hooks and Claude Code hooks call the check runner.

## Owns

`crates/bb-server/**`, `crates/bb-api/**` (only what's needed for static serving, headers and health), `xtask/src/dev.rs`, `xtask/src/build.rs`, `xtask/src/ci.rs`, `lefthook.yml`, `.claude/settings.json`, `.github/workflows/ci.yml`.

## Tasks

1. **Server** (`bugaboo serve --port <p> --web <dir>`): tokio + axum 0.8.9 + tower-http 0.7.1 + rust-embed 8.12.0 (versions from architecture §9). Release builds embed `web/app/dist`; debug builds read it from disk so `cargo build` works before the web build exists (a clear message if the folder is missing). Routes: `/health/live` (200, JSON `{"status":"ok"}`), the app (with SPA fallback to `index.html`), correct content types, `Cache-Control: immutable` for hashed assets.
2. **Security headers** on every response (SEC-11): CSP `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'; require-trusted-types-for 'script'`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: same-origin`, `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-origin`, a tight `Permissions-Policy`. (HSTS comes with TLS in M1.) Integration test that checks every header. Note: Google Fonts aren't allowed here — fonts are self-hosted later; the app uses system fonts until then.
3. **Ports** only from `tools/dev/ports.json` (read once, in one place in xtask; the server gets the port as an argument).
4. **`cargo xtask dev`**: starts Vite (port `web`, via `npm run dev -w web/app`) and the server (port `server`); stops both on Ctrl+C. **`cargo xtask build`**: web build, then release binary. **`cargo xtask ci`**: `check all`, `cargo test --workspace`, web build, `npm test`, Playwright — exactly what GitHub Actions will run later; prints a short summary.
5. **Hooks:** `lefthook.yml` — pre-commit `cargo xtask check staged`, pre-push `cargo xtask check push`, commit-msg `cargo xtask check commit-msg {1}` (if WP-03 adds that subcommand; otherwise leave a TODO with issue reference for the lead). `.claude/settings.json` — a `PostToolUse` hook for `Edit|Write|MultiEdit` that runs `cargo xtask check edit "<file>"` and returns failures to the agent. `.github/workflows/ci.yml` prepared for later: runs `cargo xtask setup` and `cargo xtask ci` on windows-latest and ubuntu-latest, triggered only by `workflow_dispatch` until the repository moves to GitHub.
6. **Register** every new crate in `docs/third-party/README.md`.

## Acceptance

- `cargo run -p bb-server -- serve --port 47200 --web <some dir>` serves files; `/health/live` answers; header test passes.
- `cargo xtask dev` starts on 47200/47201 (once WP-02's app exists; until then the server alone).
- `lefthook install` makes the hooks run; the Claude Code hook triggers on an edit.
- `cargo fmt --check`, Clippy with `-D warnings`, `cargo test --workspace`: 0 warnings, 0 failures.
