# Third-party register

Every **direct** dependency of Bugaboo — shipped or used only for building, testing or docs — has one row here, added in the same change that adds the dependency (coding-standards §5). The register check (`cargo xtask check register`, coding-standards §7) compares these tables with `Cargo.lock` and the npm lockfiles and fails on any difference.

Together with the lockfiles, this register is the source for the SBOM, the third-party notices and *About › Licenses* (COMP-01, COMP-02).

Columns:
- **Ships?** — *yes* if it ends up in the Bugaboo binary or browser app; *no* for build, test and docs tools.
- **Health** — maintained / slow / unmaintained, plus the date checked.
- **Alternatives** — what else was considered and why not.

## Tooling (npm)

| Package | Version | License | Ships? | Purpose | Health | Alternatives |
|---------|---------|---------|--------|---------|--------|--------------|
| marked | 18.0.14 | MIT | no | Turns the Markdown docs into commentable HTML pages (`tools/docs-html`). Temporary until `cargo xtask docs` uses bb-markdown. | maintained (2026-10-04) | python-markdown (needs Python on every machine); markdown-it (equivalent, more plugins we don't need) |

## Rust

| Crate | Version | License | Ships? | Purpose | Health | Alternatives |
|-------|---------|---------|--------|---------|--------|--------------|
| anyhow | 1.0.104 | MIT OR Apache-2.0 | yes (bb-server), no (xtask) | Error handling with context in the check runner | maintained (2026-10-04) | thiserror (better for libraries; the runner only reports errors) |
| clap | 4.6.7 | MIT OR Apache-2.0 | yes (bb-server), no (xtask) | Command line of `bugaboo` and `cargo xtask` | maintained (2026-10-04) | argh, pico-args (less help text, fewer checks) |
| globset | 0.4.20 | Unlicense OR MIT | no (xtask) | Matching changed files against the catalogue's `files` patterns | maintained (2026-10-04) | glob (no sets, no `{a,b}`), wildmatch |
| serde | 1.0.229 | MIT OR Apache-2.0 | no (xtask) | Reading the check catalogue and tool list | maintained (2026-10-04) | hand-written parsing (error-prone) |
| toml | 1.1.6 | MIT OR Apache-2.0 | no (xtask) | TOML parser for the catalogue and tool list | maintained (2026-10-04) | toml_edit (only needed for writing) |
| axum | 0.8.9 | MIT | yes | HTTP routing and server (ADR-0002) | maintained (2026-10-04) | actix-web (smaller tower middleware ecosystem) |
| mime_guess | 2.0.5 | MIT | yes | Content type from a file name when serving the browser app | maintained (2026-10-04) | own table (would be a copy of knowledge that exists) |
| rust-embed | 8.12.0 | MIT | yes | Embeds the built browser app in the binary (DIST-01); reads from disk in debug builds | maintained (2026-10-04) | include_dir (no debug-from-disk mode), build script with include_bytes (more code) |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | no (xtask) | Reads `tools/dev/ports.json` and the Claude Code hook input | maintained (2026-10-04) | hand-written parsing (error-prone) |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | no (tests) | Temporary folders in tests | maintained (2026-10-04) | hand-made folders in the system temp folder (leaks on failure) |
| tokio | 1.53.2 | MIT | yes | Async runtime of the server (ADR-0002) | maintained (2026-10-04) | async-std (discontinued) |
| tower | 0.5.3 | MIT | no (tests) | `oneshot` to call the router in tests without a socket | maintained (2026-10-04) | starting a real server per test (slower, needs ports) |
| tower-http | 0.7.1 | MIT | yes | Sets the security headers on every response (SEC-11) | maintained (2026-10-04) | own middleware (more code, same result) |

Server crates are added with the milestones that use them; planned ones are listed in [architecture/09-tech-stack.md](../architecture/09-tech-stack.md).

## Browser app (npm)

Filled in with the first web milestone.
