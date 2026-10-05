# ADR-0004: Authorization: one policy engine and Authorized<T>

Status: **Accepted** · 2026-10-04 · refined by [ADR-0025](0025-roles-groups-and-assignments.md)

## Context

Security is priority one. Permissions go down to single fields and single issues, and many entry points (UI, API, search, MCP, assistant, e-mail, webhooks, exports, live updates) must all obey them.

## Decision

- All decisions are made by **`bb-policy`**: permission catalogue, role assignments on the project tree with inheritance and explicit deny, field masks, visibility groups and token scopes.
- Data leaves the domain only as **`Authorized<T>`**, whose constructor is private to `bb-policy`. It carries the field mask and applies it when serialized. Returning plain domain types from `bb-app` is a compile error at the API layer.
- Searches get the actor's **visibility predicates compiled into the same SQL query**, and the parser only knows readable fields, so a hidden field looks exactly like an unknown one.
- Outbound channels (notifications, webhooks, live updates, exports, assistant context) apply the **receiver's** mask.

## Consequences

- Forgetting a check is hard: the types don't line up.
- A generated test matrix (roles × permissions × fields × entry points) runs on every build.
- Per-request caching of evaluations keeps it fast.

## Alternatives considered

- **Checks in each handler:** easy to forget one. Rejected.
- **Database row-level security:** PostgreSQL only, and field-level rules would still be needed in code. Rejected.
- **External policy engines (OPA, Cedar):** another language and runtime to secure and ship; our model is fixed and well understood. Rejected for now; the catalogue could be exported to Cedar later.

## Enforced by

ARCH-AUTHZ-ONLY, ARCH-API-OUT, ARCH-PERMS, SEC-MATRIX (see `tools/checks/catalog.toml`).
