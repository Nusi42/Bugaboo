# ADR-0002: HTTP stack: Tokio, axum, rustls

Status: **Accepted** · 2026-10-04

## Context
We need a mature async HTTP server with good middleware, HTTPS without system OpenSSL, and easy static builds for Windows and Linux.

## Decision
Use **tokio** as the runtime, **axum** for routing and extractors, **tower-http** for compression, headers and limits, and **rustls** (with `tokio-rustls`) for TLS, using the `ring` crypto provider so static musl and Windows builds need no C toolchain beyond what Rust ships. **instant-acme** handles certificates from ACME.

## Consequences
- No OpenSSL dependency; memory-safe TLS.
- axum extractors give one place to turn credentials into an actor.

## Alternatives considered
- **actix-web:** fine, but a smaller middleware ecosystem around tower.
- **aws-lc-rs provider:** stronger FIPS story, but needs CMake/C for static builds; revisit if FIPS is ever required.

## Enforced by
SEC-HEADERS, DEP-LICENSES (see `tools/checks/catalog.toml`).
