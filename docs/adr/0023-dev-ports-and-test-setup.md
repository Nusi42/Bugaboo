# ADR-0023: Development ports and test setup

Status: **Accepted** · 2026-10-04

## Context

The main development PC runs several other Vite projects (ports 5173 and 5200–5319 are taken). Default ports would clash, and Vite silently moving to another port hides the problem and breaks Playwright's server checks.

## Decision

- **Vite** builds the browser app and runs the dev server; **Vitest** runs unit tests for browser code; **Playwright** runs end-to-end and concurrency tests.
- All local ports come from **`tools/dev/ports.json`** (single source), in the range **47200–48099** (see the amendment below), which is free on the main PC and outside Windows' dynamic port range.
- Every tool uses **`strictPort: true`** (or the equivalent) and fails with a clear message naming `tools/dev/ports.json` if a port is busy.
- Playwright starts its own server instance (`e2eServer`, and one per worker from `e2eWorkersFrom`) with a fresh copy of the **sample organization** (architecture §11), so tests never touch a developer's data.
- No tool may use a framework's default port (5173, 4173, 3000, 8080, 51204).

## Consequences

- Several Bugaboo checkouts on one machine use different blocks of the range (amendment below), so they can build and test at the same time.

## Amendment (M2 WP-04): a block of ports for each work folder

Builders in different work folders (ADR-0024) ran their end-to-end tests and their throwaway PostgreSQL at the same time on the same ports, and one run answered for another. Each work folder now has its own block:

- **A block is 100 ports.** `bugaboo.winN` uses block N, which starts at `first + (N − 1) · stride` = 47200 + (N − 1) · 100: `win1` 47200–47299, `win2` 47300–47399, … `win9` 48000–48099. Nothing in 47200–48099 is reserved on the main PC, and the dynamic range starts at 49152.
- **Any other folder** (CI, a fresh clone, the master) uses block 1. The variable `BUGABOO_PORT_BLOCK` names another block (1 to 9) and wins over the name of the folder. A work folder with a number outside 1 to 9, or a variable outside it, is an error that names the variable.
- **`tools/dev/ports.json` stays the single source:** it holds `first`, `stride`, `blocks` and the *offset* of each port inside a block (`server` 0, `web` 1, … `postgres` 40). `BB_PORTS_FILE` is gone.
- **One reader per language:** `web/tooling/ports.ts` (Vite, Vitest, Playwright, the end-to-end workers) and `xtask/src/dev.rs` (`cargo xtask dev`, `db`, `ci`). Both derive the block from the name of the repository folder. Nothing else names a port; the tests that write a port as a text in a fake address never listen on it.
- Every tool still uses `strictPort`: a busy port fails and names `tools/dev/ports.json`.

Enforced by: DEV-PORTS (the layout of the file: the range, the offsets, no duplicates), the tests `dev_ports_*` in `xtask/src/dev.rs` and `dist_01_every_work_folder_has_a_block_of_its_own` in `web/tooling/ports.test.ts`.

## Alternatives considered

- **Random free ports:** avoids clashes but makes URLs unpredictable for people and documentation. Rejected.

## Enforced by

DEV-PORTS, TEST-SAMPLE-DATA, TEST-CONCURRENT (see `tools/checks/catalog.toml`).
