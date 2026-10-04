# ADR-0008: API first: OpenAPI from Rust, TypeScript client generated

Status: **Accepted** · 2026-10-04

## Context

The browser app must be a normal API client (principle 3), and API types must exist once (ODR).

## Decision

Describe every endpoint and type in Rust with **utoipa**; generate the **OpenAPI 3.1** document at build time; generate the TypeScript types and a thin fetch client with **openapi-typescript** into `web/packages/api-client`. Error codes, permission names and validation schemas are part of the document. The CI check `generated-code-freshness` fails if the client is out of date.

## Consequences

- The browser can't call what the API doesn't offer; third parties get the same API.
- API versioning: `/api/v1/…`; breaking changes need a new version and an ADR.

## Alternatives considered

- **Hand-written TypeScript types:** drift. Rejected.
- **GraphQL:** flexible, but per-field authorization and query-cost control are harder to make airtight. Rejected; `?fields=` gives lean responses.

## Enforced by

DOC-GENERATED, ARCH-WEB-API, ARCH-LEAN, ARCH-VERSIONED (see `tools/checks/catalog.toml`).
