# Architecture decision records

Each file records one decision: context, decision, consequences, alternatives. New decisions get the next number; changed decisions get a new record that supersedes the old one.

| ADR | Decision |
|-----|----------|
| [0001](0001-modular-monolith.md) | Modular monolith in one binary |
| [0002](0002-http-stack.md) | HTTP stack: Tokio, axum, rustls |
| [0003](0003-databases.md) | Databases: SQLite by default, PostgreSQL optional |
| [0004](0004-authorization.md) | Authorization: one policy engine and Authorized<T> |
| [0005](0005-search-language.md) | Search language: one Rust crate, also in the browser |
| [0006](0006-own-field-storage.md) | Own fields stored in a typed value table |
| [0007](0007-event-log.md) | Event log and outbox as the single source of change |
| [0008](0008-api-first.md) | API first: OpenAPI from Rust, TypeScript client generated |
| [0009](0009-frontend-stack.md) | Browser app: React 19, Vite, TanStack, React Aria |
| [0010](0010-design-tokens.md) | Styling through design tokens only (the styling part is replaced by 0026) |
| [0011](0011-markdown.md) | One Markdown renderer for server and browser |
| [0012](0012-sold-and-served.md) | One binary for sold and served |
| [0013](0013-ai.md) | AI: one provider interface, one tool registry, no decisions about people |
| [0014](0014-tooling.md) | One check runner for hooks, agents and CI |
| [0015](0015-personal-data-marks.md) | Personal-data marks drive GDPR features |
| [0016](0016-full-text-search.md) | Full-text search inside the database |
| [0017](0017-background-jobs.md) | Background jobs stored in the database |
| [0018](0018-settings-registry.md) | One settings registry drives storage, API and dialogs |
| [0019](0019-edit-locks.md) | Edit locks and confirmed saves |
| [0020](0020-architecture-checks.md) | Architecture is checked by many small checks, and the checks are checked |
| [0021](0021-issue-ids.md) | Issue IDs, counting or random |
| [0022](0022-who-sees-people.md) | Who sees people is part of the permission model |
| [0023](0023-dev-ports-and-test-setup.md) | Development ports and test setup |
| [0024](0024-master-repo-and-work-folders.md) | Master repository in a synced folder, work folders outside it |
| [0025](0025-roles-groups-and-assignments.md) | Roles, groups and assignments on the project tree |
| [0026](0026-one-theme-file.md) | One theme file for everything about how the app looks |
| [0027](0027-layouts-and-rules-per-issue-type.md) | Layouts, rules per issue type and the starting workflow |
| [0028](0028-message-parts.md) | The message catalog is split into parts; the first page loads one |
| [0029](0029-ai-agents.md) | AI agents: apps that act for a person, and agents that work on their own |
| [0030](0030-planning-with-people.md) | Planning with people: who works on a bar, away times, time factors and plan warnings |
| [0031](0031-workflows.md) | Workflows: statuses as data, one judge for a status change |
| [0032](0032-files.md) | Files: stored by content, checked from their content, previews in a separate process |
| [0033](0033-notifications-and-e-mail.md) | Notifications and e-mail: an outbox reader that checks again before it tells anyone |
| [0034](0034-saved-searches.md) | Saved searches: owned by one person, shared without giving anyone more |

## Template

```markdown
# ADR-NNNN: Title

Status: Proposed · date

## Context
## Decision
## Consequences
## Alternatives considered
## Enforced by

<!-- If the decision touches AI: check it against COMP-10 (no AI that ranks, evaluates or assigns people; no mood analysis). -->
<!-- If it adds a dependency: add the third-party register entry. If it stores data: add personal-data marks. -->
```
