# WP-04 — Server follow-ups from M1 and the words check

**Wave 1 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** OPS-03, OPS-08, SEC-10, SEC-15, SEC-16 (follow-up parts); glossary rule 4; follow-ups M1-F03, M1-F40, M1-F58, M1-F60, M1-F62, M1-F65, M1-F66.

## Goal

Close the M1 follow-ups that were planned for M2 and touch only the server program, the data folder and the status page, and switch on the check that keeps the glossary's *Don't say* words out of what people read.

## Owns

`crates/bb-server/src/{cli,config,data_folder,wiring}*` and their tests, `crates/bb-api/src/api/admin.rs`, `crates/bb-app/src/status.rs`, the setup-code part of `crates/bb-app/src/auth/account.rs`, `xtask/src/checks/txt_terms.rs` and `tools/checks/fixtures/TXT-TERMS/`, `docs/architecture/06-deployment.md` (the rows these tasks change).

## Tasks

1. **Certificate renewal while running (M1-F03):** a timer in the server checks the generated certificate once a day and renews it when under 30 days are left, swapping it into the TLS config without a restart; own certificates (`tls_mode = "files"`) are re-read when the files change. The status page shows the expiry it now serves.
2. **Security log that lags (M1-F40):** the status page and `bugaboo security-log verify` warn when the anchor file is ahead of the database or an entry failed to be written (the server counts failed writes since start); the warning says what to check. The decision of M1 stays: a failed entry does not refuse the sign-in. **Built differently (lead decision):** an anchor file that is ahead of the database is a hard failure, not a warning: the status page shows the reason *behind* at the level "problem" and `security-log verify` fails with it, because a restored backup and a cut-off end look the same (M2-F09). A failed write is a warning on the status page.
3. **Verifying on every visit (M1-F60):** the status endpoint verifies only the entries added since the last good check and keeps the result for a short time; a full check stays in the command. Test that a changed old entry is still found by the command.
4. **Setup code file (M1-F58):** the use case that makes the first administrator deletes `setup-code.txt` through a port that `bb-server` implements, in the same run; the start-up cleanup stays as a safety net.
5. **Password on the command line (M1-F62):** `--database-url` with a password prints a warning that names the settings file and `BUGABOO_DATABASE_URL` instead (no password in the warning).
6. **Data folder rights (M1-F65):** at every start check the whole data folder, the database file, `files/`, `backups/` and the setup-code file; report what is too open in the log and on the status page, without changing folders the person made. (The backup command stays with OPS-06 in M9.)
7. **Own certificate before the first start (M1-F66):** `bugaboo service install` accepts a data folder whose only content is the person's own files in `tls/` when named with a flag (`--take-over-data-folder`), and the message names the flag.
8. **TXT-TERMS:** read the *Don't say* column of `docs/glossary.md` and fail when such a word appears in `messages/en.json`, `userDocs/` or API names (paths, schema and property names in the API description); a word inside a longer word does not count (`login` fails, `blogging` does not). Fix existing findings.

## Acceptance tests (write first)

- `sec_10_a_certificate_near_its_end_is_renewed_without_a_restart` (with a fake clock).
- `sec_15_the_status_page_warns_when_a_log_entry_could_not_be_written` and `…_when_the_anchor_is_ahead`.
- `ops_08_the_status_page_checks_only_new_entries_and_the_command_checks_all`.
- `auth_09_the_setup_code_file_is_gone_once_the_administrator_exists`.
- `ops_10_a_database_url_with_a_password_on_the_command_line_warns`.
- `sec_16_a_too_open_data_folder_is_reported_at_start` (Unix modes; Windows access list where it can be tested).
- `ops_03_a_data_folder_with_own_certificates_is_taken_over_only_with_the_flag`.
- TXT-TERMS fails on its bad fixture, passes on the good one and on the repository.

## Checks

TXT-TERMS switched on (catalogue entry exists; implementation, fixtures). The tests above keep the start-up rules of `06-deployment.md` honest (they say *review* backed by tests today).

## Depends on

M1.
