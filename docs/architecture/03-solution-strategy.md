# Solution strategy

The few big decisions that shape everything else. Each links to its decision record.

| # | Decision | Why | ADR |
|---|----------|-----|-----|
| 1 | **A modular monolith in one binary**, with a pure domain core and adapters around it | Easiest to install, secure (one process, one boundary), fast; modules keep it maintainable | [0001](../adr/0001-modular-monolith.md) |
| 2 | **Tokio + axum + rustls** for the server | Mature, fast, memory-safe TLS, no OpenSSL to install | [0002](../adr/0002-http-stack.md) |
| 3 | **SQLite by default, PostgreSQL optional**, through sqlx and one query builder; the database schema is defined once, in Rust | Zero-setup install and a path to scale, without writing everything twice | [0003](../adr/0003-databases.md) |
| 4 | **One policy engine**; data leaves the domain only as `Authorized<T>`; searches get permission rules compiled into their SQL | Makes bypassing authorization a compile error, not a code-review hope | [0004](../adr/0004-authorization.md) |
| 5 | **Our own search language**, one Rust crate, also compiled to WebAssembly for the browser | Simple and Advanced search share one parsed form; the editor behaves exactly like the server | [0005](../adr/0005-search-language.md) |
| 6 | **Own fields stored in a typed value table** | Fast filtering and sorting on both databases, permissions per field | [0006](../adr/0006-own-field-storage.md) |
| 7 | **An append-only event log with an outbox** is the single source for history, notifications, webhooks, the security log, search indexing and live updates | One write, many consumers; nothing can change without leaving a trace | [0007](../adr/0007-event-log.md) |
| 8 | **API first:** the OpenAPI description is generated from the Rust code, and the TypeScript client from that | API types exist once (ODR); the UI can't use what the API doesn't offer | [0008](../adr/0008-api-first.md) |
| 9 | **React 19 + Vite + TanStack Router/Query + React Aria Components** in the browser | Type-safe URLs, one cache for server data, accessible building blocks | [0009](../adr/0009-frontend-stack.md) |
| 10 | **Design tokens as the only source of style**, turned into CSS variables; CSS Modules | Four themes and the theme editor from one source; customization can't inject code | [0010](../adr/0010-design-tokens.md) |
| 11 | **One Markdown renderer** (Rust), used by the server and, via WebAssembly, by the editor preview | What you preview is exactly what others see, sanitized the same way | [0011](../adr/0011-markdown.md) |
| 12 | **The same binary is sold and served**; served means one isolated instance per customer plus a control plane | No second product to maintain; strong isolation; keeps "one organization per installation" | [0012](../adr/0012-sold-and-served.md) |
| 13 | **AI through one provider interface and one tool registry**, shared by the assistant and MCP; AI never decides about people | Same rules for both ways in; AI Act high-risk uses designed out | [0013](../adr/0013-ai.md) |
| 14 | **One check runner (`cargo xtask`)** called by Git hooks, Claude Code hooks and CI; Biome and ast-grep for TypeScript and pattern rules | Checks are defined once and behave the same everywhere | [0014](../adr/0014-tooling.md) |
| 15 | **Personal-data marks in the data model** drive export, removal, deletion rules and the record of processing | GDPR can't be forgotten by a new feature | [0015](../adr/0015-personal-data-marks.md) |
| 16 | **Full-text search inside the database** (SQLite FTS5, PostgreSQL text search) | Permission rules apply in the same SQL query; no second index to secure | [0016](../adr/0016-full-text-search.md) |
| 17 | **Background jobs in the database** | No message broker to install; works on both databases | [0017](../adr/0017-background-jobs.md) |
| 18 | **One settings registry** defines every setting once: storage, defaults, permissions, API and the settings dialogs | "Everything configurable in the UI" without hand-building hundreds of forms | [0018](../adr/0018-settings-registry.md) |

## What we deliberately don't do

- No microservices, no message broker, no separate search server, no cache server. Each would add installation steps and attack surface for a scale we don't need.
- No plugins that run third-party code inside the server. Integrations use the API, webhooks and MCP.
- No server-side rendering framework. The server serves a static app and a JSON API; this keeps the security boundary simple and the CSP strict.
