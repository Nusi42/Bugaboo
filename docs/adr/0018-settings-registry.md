# ADR-0018: One settings registry drives storage, API and dialogs

Status: **Accepted** · 2026-10-04

## Context

"Everything configurable in the UI" means hundreds of settings across organization, project and person levels. Hand-building each dialog violates DRY and drifts.

## Decision

Define each setting once with a `setting!` macro: key, type, default, allowed levels (organization / project / person), who may change it, help-text message key, and whether admins can lock it. The registry generates storage access, API endpoints, OpenAPI schema and the metadata the UI uses to render settings dialogs (grouped by topic, with search).

## Consequences

- A new setting appears in the right dialog automatically, with its explanation.
- Special editors (workflow, layout, theme, permissions matrix) are still custom UI, but store their results through the same registry.

## Alternatives considered

- **Hand-written forms per setting:** slow and inconsistent. Rejected.

## Enforced by

ARCH-SETTINGS (see `tools/checks/catalog.toml`).
