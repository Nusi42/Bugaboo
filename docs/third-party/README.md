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

## Server (Rust)

Filled in with the first workspace milestone. Planned crates are listed in [architecture/09-tech-stack.md](../architecture/09-tech-stack.md).

## Browser app (npm)

Filled in with the first web milestone.
