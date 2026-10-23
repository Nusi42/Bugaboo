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

## Amendment M4 WP-16: the shape the API has at the freeze

The lead decided what the API looks like when it is frozen (the exit review, *API changes before the freeze*):

- Every enum value and every error code is written in words joined by hyphens: `in-progress`, `auth.wrong-credentials`, `workflow.not-named`. The message keys follow the same rule, so the key of a code is `error.` and the code with its dot kept.
- An issue's ID is `readable_id`, never `key`. A path parameter names the thing (`{role}`, never `{role_id}`); the groups of labels are `label_group`, the groups of statuses `status_group`, and a bare *group* is people.
- A change of a field has one shape, `change`, in the history and in notifications. The thing that is edited is `{ record: { kind, id }, part }` everywhere, with the kind from a closed list.
- A schema is written once, and a list of fields or workflows is a page like the others. The description writes each shared answer, header and paging parameter once in `components`.
- The capped lists (the previews, the sessions, the use of a field, the level tables) stay capped, with their limits as named constants; a cursor for them is an addition after the freeze.

The frozen copy is `tools/checks/api-frozen.json`, the generated description in compact form, written by `cargo xtask freeze`. WP-16 stored the first copy, so ARCH-API-FREEZE already guards the names above; the lead stores the last one at `m4-done` (and again, with a decision, for a change of a name that must stand before it). ARCH-API-LINT keeps the style: one casing for enum values, no duplicate or orphan schema, no `_id` in a path parameter. The glossary words in the names of the API are TXT-TERMS, which reads the same description.

## Alternatives considered

- **Hand-written TypeScript types:** drift. Rejected.
- **GraphQL:** flexible, but per-field authorization and query-cost control are harder to make airtight. Rejected; `?fields=` gives lean responses.

## Enforced by

DOC-GENERATED, ARCH-WEB-API, ARCH-LEAN, ARCH-VERSIONED, ARCH-API-FREEZE (the frozen copy is stored with `cargo xtask freeze` at `m4-done`), ARCH-API-LINT (see `tools/checks/catalog.toml`).
