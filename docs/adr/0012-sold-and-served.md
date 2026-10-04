# ADR-0012: One binary for sold and served

Status: **Accepted** · 2026-10-04

## Context

Bugaboo may be sold (self-hosted) or served (hosted by us); the business model isn't decided.

## Decision

- The same signed binary runs in both models (DIST-01).
- **Served** = one isolated instance per customer (own database, file store, encryption key and address) in the EU, plus a separate **control plane** for set-up, updates, backups and health (DIST-02). The control plane uses an **operator API** that cannot read customer data; emergency access needs the customer's approval and is logged in their security log (DIST-03).
- **Sold:** a signed offline license file; no phone-home and no telemetry (DIST-04).
- Export and import work between the two, both ways (DIST-06).

## Consequences

- "One organization per installation" stays true in both models; no tenant column in every table.
- Hosting cost per customer is higher than a shared multi-tenant system; accepted for isolation and simplicity.

## Alternatives considered

- **Multi-tenant shared database:** cheaper to host, but every query needs a tenant filter and one bug can leak across customers. Rejected for 1.0.

## Enforced by

review (deployment design; tested end to end in the served-model milestone), DEP-LICENSES for the license file tooling (see `tools/checks/catalog.toml`).
