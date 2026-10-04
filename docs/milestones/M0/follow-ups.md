# M0 follow-ups

Points found in the M0 reviews that were accepted for later. Each names where it will be done. Until Bugaboo tracks its own issues, this list is the issue list for M0; the IDs (`M0-F01` …) are what suppressions and TODOs refer to.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M0-F01 | Server and xtask user-facing strings are constants in code, not in `messages/en.json` | The server's message lookup comes with the API error codes | M1. Done for the server in WP-02: terminal texts, configuration, TLS and data folder messages, and the 404 body are catalog messages used through `bb_core::message` (the minimal `text::t()` lookup is gone). xtask keeps its terminal texts in code on purpose: it is a developer tool, English only, and does not depend on product crates. API error texts: WP-06 |
| M0-F02 | No Linux machine has run the code yet (Clippy checks the Linux code on Windows; tests haven't run on Linux) | Decision: not to install a Linux distribution in WSL | When the repository moves to GitHub (Linux CI runner) |
| M0-F03 | Faint and muted text are very close in Paper and Daylight (about 1.1:1 apart) | Contrast rules are met; the themes are accepted for now | M7 (theme editor) |
| M0-F04 | `web/app/index.html` `<title>` repeats the app name from the message catalog | Static HTML can't read the catalog yet | M2 (app shell) |
| M0-F05 | One frame with the device theme is possible before the chosen theme applies (deferred module script under a strict CSP) | Acceptable visually; a fix needs an inline script, which the CSP forbids | review again in M2 |
| M0-F06 | Accepted dev-only risk: npm advisory for `braces` (via stylelint), no patched version exists | Never shipped; recorded in the third-party register with a re-check date | re-check monthly |
| M0-F07 | Windows server: only Ctrl+C is handled, not console close or Ctrl+Break; POST to an unknown `/api/x` answers 405 instead of 404 | No API yet | done in M1 WP-04: console close, Ctrl+Break and shutdown stop the server cleanly; every method on an unknown `/api` path answers 404 |
| M0-F08 | Windows 8.3 short names (`/INDEX~1.HTM`) still serve files inside the app folder; percent-decoding accepts `%+1` | No traversal possible; tightening belongs with the API router | done in M1 WP-04: 8.3 short names are refused, percent escapes need exactly two hex digits |
| M0-F09 | Commit-stage checks read the working tree, not the staged content | Correct for normal use; exact staged reading needs a temporary checkout | done in M1 WP-04: commit-stage checks read an export of the staged files in `target/staged-tree`; the working tree is never written to |
| M0-F10 | DEP-REGISTER accepts a row if any locked version of that name matches | Rare (duplicate transitive versions); fix with per-package lock paths | done in M1 WP-04: each row must match the version a workspace member really depends on, from `cargo metadata` |
| M0-F11 | Some test strings in xtask break lines inside string literals instead of using `\n` | Cosmetic | next time those files change |
| M0-F12 | `setup` runs `npm ci` again if npm's hidden lockfile lacks workspace entries | Safe (only slower) | done in M1 WP-04: only keys below `node_modules/` are compared with npm's hidden lockfile |
