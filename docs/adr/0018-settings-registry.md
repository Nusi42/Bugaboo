# ADR-0018: One settings registry drives storage, API and dialogs

Status: **Accepted** · 2026-10-04

## Context

"Everything configurable in the UI" means hundreds of settings across organization, project and person levels. Hand-building each dialog violates DRY and drifts.

## Decision

Define each setting once with the `settings!` macro, which is used only in the registry file of `bb-core`: key, type, default, allowed levels (organization / project / person), who may change it, help-text message key, and whether admins can lock it. The registry generates storage access, API endpoints, OpenAPI schema and the metadata the UI uses to render settings dialogs (grouped by topic, with search).

## Consequences

- A new setting appears in the right dialog automatically, with its explanation.
- Special editors (workflow, layout, theme, permissions matrix) are still custom UI, but store their results through the same registry.

## Update (M2 WP-07, settings in layers)

- The registry also holds the **name** of each setting (a message key, next to the explanation), a **format** for text values (one of a fixed list with the words for each choice, a time zone, the address of a page of the app, the keys of shortcuts) and its **levels**. One check of a value is used by the store and the use cases.
- **Layers:** default, organization, the nearest project up the tree, person; a value the organization locked wins and lower layers are refused. *Reset to default* takes away the value of one layer. A person's own values are stored in their own table so the person is a column with a personal-data mark.
- One set of routes and one generic form serve every setting; special editors (the themes with their samples, the keys of shortcuts, the start page) draw their own control and store through the same routes.
- `appearance.theme` was replaced by `appearance.mode`, `appearance.light-theme` and `appearance.dark-theme`; migration `m0004` carries the stored values over.
- The settings that exist only for tests (feature `test-settings` of `bb-core`) are the way the project level is tested while no real setting is set on a project.

## Alternatives considered

- **Hand-written forms per setting:** slow and inconsistent. Rejected.

## Enforced by

ARCH-SETTINGS (see `tools/checks/catalog.toml`).
