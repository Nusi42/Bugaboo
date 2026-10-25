# WP-11 — No fixed paths: ask the operating system

**Wave 1 (added later) · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** none of its own; it keeps OPS-01 and OPS-04 (one program per platform, run as a service) working on every platform (coding-standards §2 "No fixed paths", a rule of the maintainer).

## Goal

No product code, tool or script writes down a path of the system. Each asks the operating system, and stops with a plain message when the system cannot answer, instead of guessing.

## Owns

`crates/bb-server/src/cli/service/**`, `crates/bb-server/src/data_folder.rs`, any other place the check finds in `crates/**`, `xtask/**`, `web/tooling/**`, `tools/**` (scripts); the new check LINT-PATHS; the dependency entries it needs (coding-standards §5, the third-party register).

## What the lead found

1. `crates/bb-server/src/cli/service/windows.rs:91` and `crates/bb-server/src/data_folder.rs:411`: `SystemRoot` with the guess `C:\Windows`, written twice. One function asks Windows for the system folder (`GetSystemDirectoryW`, through `windows-sys`, already in the tree) and both callers use it.
2. `data_folder.rs:426`: the account name read from `/etc/passwd` → `getpwuid_r` (through `libc` or `rustix`, whichever the tree already has).
3. `data_folder.rs:424`: the current account from the owner of `/proc/self` → `geteuid`.
4. `crates/bb-server/src/cli/service/systemd.rs`: `UNIT_FOLDER` `/etc/systemd/system` → the folder systemd names for units an administrator installs (`systemd-path systemd-system-conf` or `pkg-config --variable=systemdsystemconfdir systemd`, with the answer checked); `NO_LOGIN` `/usr/sbin/nologin` → found on the search path; the account and the service programs (`systemctl`, `useradd`) by name on the search path, as now.

Paths inside tests that are data for pure functions (a unit file for `/var/lib/bugaboo`, `C:\temp\shot.png` for the file-name cleaner) stay.

## Tasks

1. Look for more: `crates/**`, `xtask/**`, `web/**` (tooling, e2e support, scripts), `tools/**`, `.claude/settings.json` hooks. Report each place and what it now asks.
2. Fix the places above and those you find, each with a test of the part that can be tested without the system (the parsing of an answer, the message when the system does not answer).
3. **LINT-PATHS** through "Adding a rule" (architecture §10): an absolute path of a system in code that ships or in a tool fails, with the file, the line and the rule; tests and fixtures are left out by one named list; a bad and a good example. Then change "Enforced by: review until LINT-PATHS …" in coding-standards §2 to "Enforced by: LINT-PATHS".
4. The Linux parts cannot run on this computer (M0-F02): they build for Linux (`cargo xtask check all` does it), and their tests run where they can.

## Acceptance tests (write first)

- LINT-PATHS: the bad example (`"C:\Windows"`, `"/etc/passwd"`) fails, the good example passes.
- The system-folder function: one definition (the SSOT/DRY/ODR gate), used by both callers.
- The message when the system does not name a folder.

## Depends on

`main` at the M5 plan. It runs beside WP-02 and WP-03; it does not touch their files (WP-03 owns `xtask/src/dataset*` and the benchmark code: leave those to it and report a fixed path there to the lead).
