# WP-08 — `bugaboo setup` and the browser screens

**Wave 4 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** OPS-03, OPS-08 (status page), AUTH-09 (screens), UX-07; exit demo of M1.

## Owns

`crates/bb-server/src/cli/**` (new), `web/app/src/routes/{setup,sign-in,status}*`, their e2e tests, user docs `userDocs/install/*` and `userDocs/sign-in/*`.

## Tasks

1. **`bugaboo setup` (OPS-03):** interactive console wizard (data folder, address, HTTPS mode, database) that writes `bugaboo.toml`; non-interactive flags for scripts; `bugaboo service install|uninstall` for Windows service and systemd unit (Linux/WSL), running as a dedicated user.
2. **First run in the browser:** enter setup code → create first admin (password and/or passkey, two-step code offered) → done; plain language, works on a phone.
3. **Sign-in screens:** password, passkey, two-step code, backup code, accept invitation, sign out everywhere; errors from message keys only; keyboard and screen-reader tested (axe).
4. **Status page (OPS-08):** for admins: version, uptime, database, HTTPS certificate expiry, security-log verification result.
5. **User docs:** install on Windows, Linux, WSL; first sign-in.
6. **Tests:** Playwright on the dedicated e2e port, light/dark themes, phone viewport; screenshot set for the M1 review page.

## Acceptance

- M1 exit demo from [04-milestones.md](../../04-milestones.md) runs end to end in Playwright.
- All checks green.
