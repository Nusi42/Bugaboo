# M1 — Server core and signing in: work packages

Goal and exit demo: see [04-milestones.md](../../04-milestones.md) (M1). Rules: [`CLAUDE.md`](../../../CLAUDE.md) — **0 warnings, 0 errors, no disabled useful rules**, tests first, every new dependency registered, every new rule with a check.

M1 has more dependencies between its parts than M0, so it runs in **waves**. A wave starts when the previous one is merged to `main`.

| Wave | WP | Name | Who | Folder / branch | Status |
|------|----|------|-----|-----------------|--------|
| 1 | [WP-01](WP-01-threat-model.md) | Threat model and data-flow docs, plan | Lead | `bugaboo.win1` / `WIN1` | merged |
| 1 | [WP-02](WP-02-core-foundations.md) | Core foundations: IDs, error codes, personal-data marks, events, settings, messages in Rust | Builder | `bugaboo.win2` / `WIN2` | fixing |
| 1 | [WP-03](WP-03-database-tooling.md) | Database tooling: local PostgreSQL, test harness for both databases | Builder | `bugaboo.win3` / `WIN3` | merged |
| 1 | [WP-04](WP-04-server-hardening.md) | Server hardening: HTTPS, security headers, one binary, start-up config, health, logs, M0 follow-ups | Builder | `bugaboo.win4` / `WIN4` | merged |
| 2 | [WP-05](WP-05-store.md) | Store: schema once for both databases, event log and outbox, jobs, security log, secrets | Builder | `bugaboo.win2` / `WIN2` | waiting |
| 2 | [WP-06](WP-06-api-skeleton.md) | API skeleton: OpenAPI, generated TypeScript client, problem+json, rate limits | Builder | `bugaboo.win3` / `WIN3` | waiting |
| 3 | [WP-07](WP-07-sign-in.md) | Signing in: passwords, sessions, CSRF, two-step codes, passkeys, invitations, setup code | Builder | `bugaboo.win4` / `WIN4` | waiting |
| 4 | [WP-08](WP-08-setup-and-sign-in-screens.md) | `bugaboo setup` and the browser screens: first run, sign-in, status | Builder | `bugaboo.win5` / `WIN5` | waiting |

Status values: waiting → building → reviewing → fixing → merged. The lead updates this column with every merge, so a new session can see where things stand.

**File ownership** works as in M0: a package changes only what it owns; shared files get added lines only; the lead resolves conflicts.

**Every builder starts by resetting its folder to the current `main`:** `git fetch origin && git checkout -B WINn origin/main`.

**Follow-ups from M0** handled here: M0-F01 (WP-02, WP-06), M0-F07, M0-F08 (WP-04), M0-F09, M0-F10, M0-F12 (WP-04, tooling).
