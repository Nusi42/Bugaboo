# ADR-0001: Modular monolith in one binary

Status: **Accepted** · 2026-10-04

## Context

Bugaboo must be secure, fast and installable on Windows, Linux and WSL by people without operations experience. The scale target (500 people, 1 M issues) fits one machine.

## Decision

Build one Rust binary containing the API, the embedded browser app, background jobs and the MCP server. Inside, use a **hexagonal** structure: a pure domain core (`bb-core`, `bb-policy`, `bb-searchlang`, `bb-markdown`), use cases (`bb-app`) that depend only on ports (traits), and adapters (`bb-store`, `bb-files`, `bb-ai`, `bb-notify`, `bb-api`, `bb-mcp`). Module boundaries are crates, and dependency rules are checked by `cargo xtask check only ARCH-DEPS`.

## Consequences

- One process, one security boundary, one thing to install, back up and update.
- Use cases are testable with in-memory fakes; adapters are tested separately.
- Scaling beyond one machine would need work later (accepted: not a 1.0 goal).

## Alternatives considered

- **Microservices:** more moving parts, more attack surface, harder installation. Rejected.
- **One big crate:** faster to start, but boundaries erode and compile times grow. Rejected.

## Enforced by

ARCH-DEPS, ARCH-HTTP-EDGE, ARCH-MAP-SYNC, META-NEW-MODULE (see `tools/checks/catalog.toml`).
