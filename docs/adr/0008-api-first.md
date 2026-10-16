# ADR-0008: API first: OpenAPI from Rust, TypeScript client generated

Status: **Accepted** · 2026-10-04

## Context

The browser app must be a normal API client (principle 3), and API types must exist once (ODR).

## Decision

Describe every endpoint and type in Rust with **utoipa**; generate the **OpenAPI 3.1** document at build time; generate the TypeScript types and a thin fetch client with **openapi-typescript** into `web/packages/api-client`. Error codes, permission names and validation schemas are part of the document. The CI check `generated-code-freshness` fails if the client is out of date.

## Consequences

- The browser can't call what the API doesn't offer; third parties get the same API.
- API versioning: `/api/v1/…`; breaking changes need a new version and an ADR.

## Amendment M4: the API freeze

Decision M4-Q8, when the API stops changing names:

- Until `m4-done`, the only client is Bugaboo's own web app, generated from the same document, so names may still change. M4 WP-02 brings every name in line with the glossary before the freeze.
- From `m4-done` on, `/api/v1` is frozen. Renaming or removing anything is a breaking change and needs a decision record and a new version:
  - a path, an operation ID, a schema, a property or a parameter;
  - an enum value or an error code.
- Adding a route, an optional property, an enum value the client must already accept, or an error code stays free.
- At `m4-done` the lead stores the generated document as the frozen copy. A check then compares every new document with it and fails on anything removed or renamed (ARCH-API-FREEZE).

## Alternatives considered

- **Hand-written TypeScript types:** drift. Rejected.
- **GraphQL:** flexible, but per-field authorization and query-cost control are harder to make airtight. Rejected; `?fields=` gives lean responses.

## Enforced by

DOC-GENERATED, ARCH-WEB-API, ARCH-LEAN, ARCH-VERSIONED, ARCH-API-FREEZE (planned, switched on at `m4-done`; see `tools/checks/catalog.toml`).
