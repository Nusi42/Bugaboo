# WP-04 — Server hardening: HTTPS, start-up config, health, logs

**Wave 1 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** SEC-10, SEC-11, OPS-01, OPS-08 (server part), OPS-10, PRIV-10 (TLS part), DIST-01; ADR-0002, ADR-0012; follow-ups M0-F07, M0-F08, M0-F09, M0-F10, M0-F12.

## Owns

`crates/bb-server/**`, `crates/bb-api/src/{web,headers,router}*` and new `tls`/`config` modules, xtask follow-up fixes (`checks/dep_register.rs`, staged-content reading in `repo.rs`/runner, `setup.rs` npm check).

## Tasks

1. **Start-up config (OPS-10):** `bugaboo.toml` + environment variables (`BUGABOO_…`): address, port, data folder, database URL, TLS mode, trusted proxies. Everything else comes from the settings registry later. Clear plain-language errors for bad values.
2. **HTTPS (SEC-10):** rustls with the `ring` provider. Modes: `self-signed` (generated with rcgen on first start, stored in the data folder, renewed before expiry), `files` (cert + key paths), `off` — allowed only when binding to 127.0.0.1/::1 or when `trusted_proxies` is set. ACME is V1 (not here).
3. **Security headers (SEC-11):** one header layer for every response: CSP with no inline script or style (extend M0's), `require-trusted-types-for 'script'`, HSTS on HTTPS, `frame-ancestors 'none'`, COOP/CORP, `nosniff`, a tight Permissions-Policy, `Referrer-Policy: same-origin` — values as in architecture §7 (Browser protections); response compression (gzip/brotli) for text. New check **SEC-HEADERS** (catalogue entry active, bad/good examples): an integration test asserts the full header set on a page, an asset, an API error and a 404.
4. **One binary (OPS-01, DIST-01):** `cargo xtask build --release` produces the Windows x64 binary with the web app embedded; add `aarch64-pc-windows-msvc` to the toolchain targets if it builds on this PC, otherwise record a follow-up. Linux x64/ARM64 static builds are prepared in the CI workflow (they run once the repository is on GitHub, M0-F02). No edition or model switch in code: behaviour differences come from configuration only.
5. **Data folder layout** (architecture §6): `bugaboo.db`, `files/`, `keys/` (created with owner-only permissions; on Windows ACLs restricted to the service account/user), `backups/`, `tls/`.
6. **Health and logs (OPS-08):** `/health/live`, `/health/ready` (checks the data folder now, database once WP-05 lands); structured JSON logs via tracing with request IDs; no personal data in logs (IDs only; IP addresses truncated); DATA-LOGS check (ast-grep or code pattern: no names/e-mails/free text in tracing macros).
7. **M0 follow-ups:** F07 (Windows console close / Ctrl+Break; unknown API paths → 404 for every method), F08 (reject 8.3 short names; strict two-hex-digit percent decoding), F09 (commit-stage checks read staged content), F10 (DEP-REGISTER matches the exact locked package of each direct dependency), F12 (npm install check uses only `node_modules/` keys).
8. **Register** new crates (rustls 0.23.45, tokio-rustls 0.26.6, rcgen 0.14.10, tracing-subscriber 0.3.23, …).

## Acceptance

- Integration tests: HTTPS with a generated certificate (full header set (SEC-HEADERS)), refusal of `tls = "off"` on a public address, config errors, health endpoints, log lines without personal data.
- All checks green, Clippy for Windows and Linux 0 warnings.
