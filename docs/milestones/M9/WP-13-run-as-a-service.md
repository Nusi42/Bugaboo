# WP-13 — Run as a service, proven on a clean Windows and a Linux with systemd

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4` · **Model:** Sonnet (an exact brief: the commands exist; this package proves them step by step)
**Features:** OPS-04. Architecture §6, *Commands*; follow-ups M1-F57, M1-F63, M4-F61.

## Goal

`bugaboo service install`, a start, an upgrade and `bugaboo service uninstall` are run for real: on a clean Windows in Windows Sandbox, and in a fresh WSL distribution with systemd. The systemd unit is hardened as far as the server still starts, the services do not restart in a loop after an upgrade stops, and a company mail server with its own certificate authority can be reached.

## Owns

`crates/bb-server/src/cli/service/` (`windows.rs`, `systemd.rs`, `mod.rs`: the hardening of the unit, the failure actions, `bugaboo service status`); the setting `mail.certificate-authority` (a PEM file in the data folder's `tls/`, read with the mail server's settings) in `crates/bb-core/src/settings/` and its use in `crates/bb-notify/` (M4-F61); the try-out scripts in `xtask/src/try_install/` (new: `cargo xtask try-install windows-sandbox` writes a Windows Sandbox configuration into `.work/try-install/` that maps the release folder read only and a results folder, and runs the install script inside as the sandbox's administrator; `cargo xtask try-install wsl` imports a fresh distribution into `.work/try-install/wsl/`, switches systemd on inside it, runs the same steps, and unregisters it at the end); `userDocs/install/windows.md`, `linux.md`, `wsl.md` (brought in line with what ran).

## What is decided

- **The clean computers** (M9-Q17 as answered): Windows Sandbox, after the maintainer switches the feature on once; a WSL distribution imported for the run from the published Ubuntu image, downloaded into `.work/`, with systemd switched on in its own `/etc/wsl.conf`, and unregistered after (the existing WSL distribution of the computer is never changed). The steps are one list run in both: verify the program (`bugaboo update verify`), `bugaboo setup --non-interactive`, `bugaboo service install`, the server answers on its port, the setup code is there, `bugaboo service status`, replace the program with the next build and restart (the backup of WP-07 is made, migrations run), `bugaboo service uninstall` (the data stays). Each step writes its result to the results folder; the run fails at the first failed step.
- **The account of the Windows service** (M1-F57): the install checks that `NT SERVICE\Bugaboo` can read the program's folder and refuses with a plain sentence when it cannot (a program under a user's profile); `keys export`, `rotate-key` and `security-log verify` for the service's data folder are documented as run from an administrator's console with the service stopped, and the commands say so.
- **The systemd unit** (M1-F63): `ProtectHome=yes`, `PrivateDevices=yes`, `CapabilityBoundingSet=` (empty), `RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX`, `NoNewPrivileges=yes`, `ProtectSystem=strict`, `ReadWritePaths=` the data folder, `PrivateTmp=yes`, `Restart=on-failure` with `RestartPreventExitStatus=` the upgrade stop's code (WP-07); each line kept only when the run in WSL shows the server still starts and serves; a line that has to go is written down with the reason.
- **The Windows service's failure actions:** restart after a failure, except after the upgrade stop's exit code.
- **The mail server's authority** (M4-F61): an administrator puts the authority's PEM file into `tls/` and names it in *E-mail*; Bugaboo trusts it besides the built-in roots for the mail server only.
- **What is left for the maintainer:** switching on Windows Sandbox once; an optional run on a real clean PC from the same install guide.

## Tasks

1. Write the acceptance tests below.
2. The hardened unit, the failure actions, `service status`, the check of the service account's rights.
3. `cargo xtask try-install windows-sandbox` and `wsl`.
4. The runs in both, with their results kept in `.work/` and summarized in the package's report.
5. The mail server's authority.
6. The install guides brought in line with the runs.

## Acceptance tests (write first)

- `ops_04_the_unit_holds_the_hardening_lines` (the text of the unit, as today's tests).
- `ops_04_a_stopped_upgrade_does_not_restart_the_service` (the unit's and the Windows service's settings).
- `ops_04_the_install_refuses_a_program_the_service_account_cannot_read`.
- `ops_04_service_status_names_the_state`.
- `ops_04 the service installs starts upgrades and uninstalls in a clean Windows` (the try-out run in Windows Sandbox; its results file).
- `ops_04 the service installs starts upgrades and uninstalls in a fresh Linux with systemd` (the try-out run in WSL).
- `ntf_02_a_mail_server_with_its_own_authority_is_reached_over_tls` (a test mail server with a certificate of a test authority).

## Checks

LINT-PATHS (no fixed path in the scripts: the sandbox and WSL tools found through the system, the folders asked of it), SEC-PRIVATE (no name of this computer or its distributions in a file or a log kept), ARCH-SETTINGS, TXT-CATALOG, DEP-REGISTER.

## Review

Code review with the SSOT/DRY/ODR gate: one list of steps for both systems, the unit text written once. **Security review:** the service account, the unit's hardening, the mail authority trusted for the mail server only.

## Depends on

WP-07 and WP-08 merged (an upgrade with its backup, a signed program to verify). Runs beside WP-10, WP-11 and WP-12.
