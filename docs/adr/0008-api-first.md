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
- Dates and times are texts of RFC 3339 (`format: date-time`), the names of operations say what they do (`getDraft`, `createAssignment`, `markNotificationsRead`), and the enums that are no body have no `Body` in their names. A change of a field has one shape, `change`, in the history and in notifications. The thing that is edited is `{ record: { kind, id }, part }` everywhere, with the kind from a closed list.
- A schema is written once, and a list of fields or workflows is a page like the others. The description writes each shared answer, header and paging parameter once in `components`.
- A route that makes something answers `200` with the thing it made, and no `Location` header: the browser needs the body, and a client finds the thing by its ID. This is the decision of B7; a `201` would be a break after the freeze, so it stays.
- The capped lists (the previews, the sessions, the use of a field, the level tables) stay capped, with their limits as named constants; a cursor for them is an addition after the freeze.

The frozen copy is `tools/checks/api-frozen.json`, the generated description in compact form, written by `cargo xtask freeze`. WP-16 stored the first copy, so ARCH-API-FREEZE already guards the names above; the lead stores the last one at `m4-done` (and again, with a decision, for a change of a name that must stand before it). ARCH-API-LINT keeps the style: one casing for enum values, no duplicate or orphan schema, no `_id` in a path parameter. The glossary words in the names of the API are TXT-TERMS, which reads the same description.

### Kept in v1 for good (decided by the lead, WP-16)

- **The server status.** `/api/v1/status`, `ServerStatus` and its routes keep their names. A health route is conventional, and the glossary has a row for *server status*, so it is not the status of an issue.
- **The paths of previews and of the person.** The five patterns of the routes of previews stay. The rule for a new route: `/auth` is signing in and credentials, `/me` is one's own preferences.
- **`RecordKind` is closed.** It lists 26 kinds of record, and the use cases refuse those that cannot be edited (the answer is `resource.not-found`). The enum only grows. A word that is not in it (also the form with an underscore) is refused as an invalid request, `400` `request.invalid`, in a body and in an address.

### Open and closed (decided by the lead, WP-16)

- **Open.** Every enum of an answer, every `kind` union, every `oneOf` variant and every error code is open: a client must accept a value it does not know, and a new value there is an addition. No answer enum is marked closed today.
- **Closed.** What a request names is closed to the server: the values of an enum in a request (`RecordKind`, a level, a setting type) are refused when unknown, and ARCH-API-FREEZE refuses to take one away or to put a rule on a text that had none.

### Kept in v1 for good, schemas (decided by the lead, WP-16)

- **The `Page_*` names.** The 26 schemas that utoipa names `Page_X` for the page of a list of `X` keep these names. They are the name of a page of an item, and the client types are made from them.
- **`IssueItem`.** The six operations that answer `IssueItem` (`changeIssue`, `saveIssueDescription`, `saveIssueFieldText`, `moveIssue`, `restoreIssue`, `createIssue`) keep it: its members are always there when no `fields=` is sent, so the optional types are only the type of the list.
- **The tables of levels.** `FieldLevelTableBody`, `PeopleLevelTableBody` and their rows, cells and answers have plain names (a macro writes one struct for each kind of subject), so there is no generic schema name left.

### Compatible later (not in the freeze)

These are additions after the freeze and break no client: optional and nullable members that are always there in new bodies (M4-F174), the per-route list of error codes and the wording of the shared `400` and `409` answers (B2), the open `fields` maps with inline field keys (B11, M4-F177), cursors for the capped lists (M4-F172), a stable cursor in the store for the lists of fields and workflows (M4-F171) and a `truncated` member of the list of workflows (M4-F180), and an optional `base_version` in the body of a change of a person's own setting and a `version` member in the answer (M5-F97, ADR-0018).

## Alternatives considered

- **Hand-written TypeScript types:** drift. Rejected.
- **GraphQL:** flexible, but per-field authorization and query-cost control are harder to make airtight. Rejected; `?fields=` gives lean responses.

## Enforced by

DOC-GENERATED, ARCH-WEB-API, ARCH-LEAN, ARCH-VERSIONED, ARCH-API-FREEZE (the frozen copy is stored with `cargo xtask freeze` at `m4-done`), ARCH-API-LINT (see `tools/checks/catalog.toml`).
