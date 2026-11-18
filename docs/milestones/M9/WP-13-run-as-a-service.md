# WP-13 — Run as a service, proven on a clean Windows and a Linux with systemd

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4` · **Model:** Opus (service accounts, unit hardening and real systems: open questions turn up while it runs)
**Features:** OPS-04. Architecture §6, *Commands*; follow-ups M1-F57, M1-F63.

## Goal

`bugaboo service install`, a start, an upgrade and `bugaboo service uninstall` are run for real: on a clean Windows in Windows Sandbox, and in a fresh WSL distribution with systemd. The systemd unit is hardened as far as the server still starts, and the services do not restart in a loop after an upgrade stops.

## Owns

`crates/bb-server/src/cli/service/` (`windows.rs`, `systemd.rs`, `mod.rs`: the hardening of the unit, the failure actions of both services, which read WP-07's exit code of an upgrade stop, `bugaboo service status`); the try-out scripts in `xtask/src/try_install/` (new: `cargo xtask try-install windows-sandbox` starts Windows Sandbox through its command-line tool `wsb` (Windows 11 24H2 and later; found through the system, never a fixed path), shares the release folder read only and a results folder from `.work/try-install/`, and runs the install script inside as the sandbox's administrator, falling back to a `.wsb` configuration file where `wsb` is missing; `cargo xtask try-install wsl` imports a fresh distribution into `.work/try-install/wsl/` from the published Ubuntu image, downloaded once and cached in `.work/try-install/images/` with its checksum, switches systemd on inside it, runs the same steps, and unregisters it at the end); `userDocs/install/windows.md`, `linux.md`, `wsl.md` (brought in line with what ran).

## What is decided

- **The clean computers** (M9-Q17): Windows Sandbox, after the maintainer switches the feature on once; a WSL distribution imported for the run from the published Ubuntu image, downloaded into `.work/`, with systemd switched on in its own `/etc/wsl.conf`, and unregistered after (the existing WSL distribution of the computer is never changed). The steps are one list run in both: verify the program (`bugaboo update verify`), `bugaboo setup --non-interactive`, `bugaboo service install`, the server answers on its port, the setup code is there, `bugaboo service status`, replace the program with the next build and restart (the backup of WP-07 is made, migrations run), `bugaboo service uninstall` (the data stays). Each step writes its result to the results folder; the run fails at the first failed step.
- **The account of the Windows service** (M1-F57): the install checks that `NT SERVICE\Bugaboo` can read the program's folder and refuses with a plain sentence when it cannot (a program under a user's profile); `keys export`, `rotate-key` and `security-log verify` for the service's data folder are documented as run from an administrator's console with the service stopped, and the commands say so.
- **The systemd unit** (M1-F63): `ProtectHome=yes`, `PrivateDevices=yes`, `CapabilityBoundingSet=` (empty), `RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX`, `NoNewPrivileges=yes`, `ProtectSystem=strict`, `ReadWritePaths=` the data folder, `PrivateTmp=yes`, `Restart=on-failure` with `RestartPreventExitStatus=` the upgrade stop's code (WP-07); each line kept only when the run in WSL shows the server still starts and serves; a line that has to go is written down with the reason.
- **The Windows service's failure actions:** restart after a failure, except after the upgrade stop's exit code.
- **What is left for the maintainer:** switching on Windows Sandbox once; an optional run on a real clean PC from the same install guide.

## Tasks

1. Write the acceptance tests below.
2. The hardened unit, the failure actions, `service status`, the check of the service account's rights.
3. `cargo xtask try-install windows-sandbox` and `wsl`.
4. The runs in both, with their results kept in `.work/` and summarized in the package's report.
5. The install guides brought in line with the runs.

## Acceptance tests (write first)

- `ops_04_the_unit_holds_the_hardening_lines` (the text of the unit, as today's tests).
- `ops_04_a_stopped_upgrade_does_not_restart_the_service` (the unit's and the Windows service's settings).
- `ops_04_the_install_refuses_a_program_the_service_account_cannot_read`.
- `ops_04_service_status_names_the_state`.
- `ops_04 the service installs starts upgrades and uninstalls in a clean Windows` (the try-out run in Windows Sandbox; its results file).
- `ops_04 the service installs starts upgrades and uninstalls in a fresh Linux with systemd` (the try-out run in WSL).

## Checks

LINT-PATHS (no fixed path in the scripts: the sandbox and WSL tools found through the system, the folders asked of it), SEC-PRIVATE (no name of this computer or its distributions in a file or a log kept), ARCH-SETTINGS, TXT-CATALOG, DEP-REGISTER.

## Review

Code review with the SSOT/DRY/ODR gate: one list of steps for both systems, the unit text written once. **Security review:** the service account and the unit's hardening.

## Depends on

WP-07 and WP-08 merged (an upgrade with its backup, a signed program to verify); Windows Sandbox switched on by the maintainer for the Windows half. Runs beside WP-10 and WP-11.
