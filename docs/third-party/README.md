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
| marked | 18.0.14 | MIT | no | Turns the Markdown docs into commentable HTML pages (`tools/docs-html`). Temporary until `cargo xtask docs` uses bb-markdown. | maintained (2026-10-04) | python-markdown (needs Python on every machine); markdown-it (equivalent, more plugins we don't need) |

## Rust

| Crate | Version | License | Ships? | Purpose | Health | Alternatives |
|-------|---------|---------|--------|---------|--------|--------------|
| anyhow | 1.0.104 | MIT OR Apache-2.0 | no (xtask) | Error handling with context in the check runner | maintained (2026-10-04) | thiserror (better for libraries; the runner only reports errors) |
| clap | 4.6.7 | MIT OR Apache-2.0 | no (xtask) | Command line of `cargo xtask` | maintained (2026-10-04) | argh, pico-args (less help text, fewer checks) |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | no (xtask) | Unpacking the `.tar.gz` archives of downloaded tools in `cargo xtask setup` | maintained (2026-10-04) | system `tar` (differs between Windows and Linux) |
| globset | 0.4.20 | Unlicense OR MIT | no (xtask) | Matching changed files against the catalogue's `files` patterns | maintained (2026-10-04) | glob (no sets, no `{a,b}`), wildmatch |
| serde | 1.0.229 | MIT OR Apache-2.0 | no (xtask) | Reading the check catalogue and tool list | maintained (2026-10-04) | hand-written parsing (error-prone) |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | no (xtask) | Reading `cargo metadata` and `package.json` / `package-lock.json` in the dependency and architecture checks | maintained (2026-10-04) | hand-written parsing (error-prone) |
| sha2 | 0.11.0 | MIT OR Apache-2.0 | no (xtask) | Verifying the sha256 of every downloaded tool before it is unpacked | maintained (2026-10-04) | ring (large, C and assembly code); calling `certutil` / `sha256sum` (differs per system) |
| tar | 0.4.46 | MIT OR Apache-2.0 | no (xtask) | Reading the `.tar.gz` archives of downloaded tools | maintained (2026-10-04) | system `tar` (differs between Windows and Linux) |
| toml | 1.1.6 | MIT OR Apache-2.0 | no (xtask) | TOML parser for the catalogue and tool list | maintained (2026-10-04) | toml_edit (only needed for writing) |
| zip | 8.6.0 | MIT | no (xtask) | Reading the `.zip` archives of downloaded tools on Windows | maintained (2026-10-04) | PowerShell `Expand-Archive` (not on Linux) |

Server crates are added with the milestones that use them; planned ones are listed in [architecture/09-tech-stack.md](../architecture/09-tech-stack.md).

## Browser app (npm)

Filled in with the first web milestone.
