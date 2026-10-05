# ADR-0022: Who sees people is part of the permission model

Status: **Accepted** · 2026-10-04 · refined by [ADR-0025](0025-roles-groups-and-assignments.md)

## Context

Names of people appear in many places: reported by, assigned to, last changed by, comment and history authors, followers, "… is editing", time entries, mentions, avatars, e-mails, exports, the assistant. Under GDPR, organizations may need to hide who did what from some roles (for example external guests, or works-council agreements).

## Decision

- **Who sees people** is configured per role on the project tree, like field permissions, for each kind of people information: **Names**, **Role only** (*"a Team member"*) or **Hidden** (*"Someone"*).
- Every place that shows a person uses one type, **`PersonRef`**, which is resolved for the **viewer** by `bb-policy` into a name, a role label or *Someone*. Raw person data never reaches serialization directly.
- If a viewer can't see names for a kind of information, they also can't search, sort, group or filter by it; the field behaves like a hidden field (SEC-06).
- Edit-lock notices follow the same rule: *"Someone is editing the description"*.
- Mentions: people who can't see names can only mention groups or roles.

## Consequences

- One resolution point; history, notifications, exports, MCP and the assistant automatically obey it.
- A tiny cost per rendered person (cached per request).

## Alternatives considered

- **Treat each people field as an ordinary field only:** misses comment authors, history, presence and avatars. Rejected.

## Enforced by

ARCH-PERSON-REF, SEC-MATRIX (see `tools/checks/catalog.toml`).
