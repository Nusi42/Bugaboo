# Architecture

Status: **Accepted** · Phase 3 of 5 · 2026-10-04

The structure follows the arc42 template (shortened) with C4-style diagrams as Mermaid text, so diagrams live and change with the code. Decision records are in [`../adr/`](../adr/). This folder is part of the CRA technical documentation (COMP), so it must stay current; the hooks check that (coding-standards §5).

| # | Chapter |
|---|---------|
| 1 | [Goals and constraints](01-goals-and-constraints.md) |
| 2 | [Context](02-context.md) |
| 3 | [Solution strategy](03-solution-strategy.md) |
| 4 | [Building blocks](04-building-blocks.md) |
| 5 | [Runtime](05-runtime.md) |
| 6 | [Deployment](06-deployment.md) |
| 7 | [Cross-cutting concepts](07-cross-cutting.md) |
| 8 | [Quality scenarios and risks](08-quality-and-risks.md) |
| 9 | [Tech stack](09-tech-stack.md) |
| 10 | [Architecture checks](10-checks.md) |
| 11 | [Testing, sample data and local ports](11-testing.md) |
| 12 | [Security: threat model and personal-data flows](12-security.md) |

The commentable HTML version (`../03-architecture.html`) is **generated** from these files and the ADRs by `cargo xtask docs` — edit the Markdown, never the HTML.
