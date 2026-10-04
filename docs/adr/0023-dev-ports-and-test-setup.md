# ADR-0023: Development ports and test setup

Status: **Accepted** · 2026-10-04

## Context

The main development PC runs several other Vite projects (ports 5173 and 5200–5319 are taken). Default ports would clash, and Vite silently moving to another port hides the problem and breaks Playwright's server checks.

## Decision

- **Vite** builds the browser app and runs the dev server; **Vitest** runs unit tests for browser code; **Playwright** runs end-to-end and concurrency tests.
- All local ports come from **`tools/dev/ports.json`** (single source), in the block **47200–47299**, which is free on the main PC and outside Windows' reserved and dynamic port ranges.
- Every tool uses **`strictPort: true`** (or the equivalent) and fails with a clear message naming `tools/dev/ports.json` if a port is busy.
- Playwright starts its own server instance (`e2eServer`, and one per worker from `e2eWorkersFrom`) with a fresh copy of the **sample organization** (architecture §11), so tests never touch a developer's data.
- No tool may use a framework's default port (5173, 4173, 3000, 8080, 51204).

## Consequences

- Several Bugaboo checkouts on one machine need different `ports.json` values; the file can be overridden by `BB_PORTS_FILE`.

## Alternatives considered

- **Random free ports:** avoids clashes but makes URLs unpredictable for people and documentation. Rejected.

## Enforced by

DEV-PORTS, TEST-SAMPLE-DATA, TEST-CONCURRENT (see `tools/checks/catalog.toml`).
