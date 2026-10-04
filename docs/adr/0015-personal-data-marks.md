# ADR-0015: Personal-data marks drive GDPR features

Status: **Accepted** · 2026-10-04

## Context
GDPR features (export, removal, keep-and-delete rules, record of processing) break silently when a new feature adds personal data and forgets them.

## Decision
Every stored field is marked in its entity definition with a derive attribute, for example `#[personal(category = "employee_time", basis = "legal_obligation")]` or `#[not_personal]`. Unmarked fields fail the build. A generated **personal-data map** drives *Download my data*, the *Remove a person* plan, keep-and-delete jobs, the record of processing (Art. 30) and the data-flow map.

## Consequences
- GDPR compliance becomes part of the type system and CI, not a checklist.
- Events and logs follow the same marks.

## Alternatives considered
- **A hand-maintained spreadsheet of personal data:** drifts. Rejected.

## Enforced by
DATA-MARKS, DATA-LOGS (see `tools/checks/catalog.toml`).
