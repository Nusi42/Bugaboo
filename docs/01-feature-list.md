# 01 — Feature List

Status: **Complete** · Phase 1 of 5 · Last updated 2026-10-04

This is the full feature catalogue for Bugaboo. Names follow the [glossary](glossary.md), which is the single source for all naming: plain language, no terms copied from other vendors. Every feature has a stable ID (used later by
milestones, tests and ADRs) and a tier:

| Tier   | Meaning                                                              |
|--------|----------------------------------------------------------------------|
| **M**  | MVP — needed for the first usable release                            |
| **V1** | Required for 1.0                                                     |
| **L**  | Later — designed for now (no architectural dead ends), built after 1.0 |

---

## 0. Guiding principles

Priority order: **security → UX → AI → architecture → feature completeness → installation**.
What each means in practice:

1. **One authorization path.** A single server-side policy engine decides every read and
   write. The web UI, REST API, search language, MCP server, AI assistant, exports,
   notifications, webhooks and search all go through it. Nothing gets a "trusted" side door.
2. **Deny by default.** A new role, field, project or integration can do nothing until
   someone grants it access.
3. **The UI is a normal API client.** The React app uses only the public, documented API.
   This means the UI cannot do anything the API doesn't allow, and the API can do
   everything the UI does.
4. **Everything is configurable in the UI.** Settings are stored in the database and edited
   in dialogs. Files (YAML/JSON) are only for export, import, backup and bootstrap.
5. **Constrained customization.** Themes and custom styles are validated design tokens, not
   raw CSS or HTML, so customization cannot become an injection vector.
6. **One binary.** The server ships as a single self-contained executable with the web UI
   embedded, and sensible defaults.
7. **Lean and fast.** The browser receives only the data a screen needs and the user may
   see. Performance budgets are enforced automatically (see PERF).
8. **Engineering rules.** SSOT, DRY and ODR are mandatory and enforced by hooks. See
   [coding-standards.md](coding-standards.md).
9. **Plain for everyone.** The UI is designed for people less technical than the average
   person. Plain words everywhere (see [glossary](glossary.md)), and no names copied from
   other vendors. Technical words are used only when they are genuinely common.
10. **Privacy by design.** GDPR is part of every feature, not an add-on: collect little,
    show little, delete on schedule, and let people see and take their own data (see PRIV).

---

## 1. Security & access control (SEC)

### Permission model

- **SEC-01 (M)** — Central policy engine: every action is checked as *(subject, action, resource, field)*. Default deny.
- **SEC-02 (M)** — **Users**, **groups** (nested groups are L), **roles** (named bundles of permissions) and **role assignments** at a scope: global, project, or project subtree.
- **SEC-03 (M)** — Permission inheritance down the project hierarchy, with explicit per-project overrides (grant and *deny*), shown in a **Check access** view that explains every result.
- **SEC-04 (M)** — **Field-level permissions** for every field, built-in or custom: *hidden / read / write* per role, optionally per issue type.
- **SEC-05 (V1)** — **Restricted issues** (issue *visibility*): limit single issues (for example, a vulnerability report) to chosen groups, even inside a project the user can otherwise see.
- **SEC-06 (M)** — Field permissions apply everywhere a value could leak:
  - responses, change history, comments, exports, notifications/e-mails, webhooks, AI context, MCP results;
  - search filters, sorting, grouping, totals and autocomplete. You cannot filter on a field you cannot read, because that would reveal its value through the results.
- **SEC-07 (V1)** — **See as …**: admins view Bugaboo exactly as a chosen person or role sees it (read-only, recorded in the security log).
- **SEC-08 (M)** — Built-in starting roles: *Administrator, Project lead, Team member, Contributor, Viewer, Guest*. They are editable copies, never hard-coded.

### Platform hardening

- **SEC-10 (M)** — HTTPS by default:
  - built-in TLS (rustls) using an auto-generated certificate, a provided certificate or ACME/Let's Encrypt (V1);
  - plain HTTP only on localhost or behind an explicitly configured trusted proxy.
- **SEC-11 (M)** — Strict browser security headers:
  - a CSP with no inline script or style;
  - Trusted Types, HSTS, `frame-ancestors 'none'`, COOP/CORP, `nosniff`, a tight Permissions-Policy.
- **SEC-12 (M)** — Sessions use `__Host-` cookies (HttpOnly, Secure, SameSite). The server adds CSRF protection, rotates the session on privilege change, enforces idle and absolute timeouts, and lets users list and revoke sessions.
- **SEC-13 (M)** — All user-written rich text (Markdown) is sanitized with an allowlist. Raw HTML, `javascript:` URLs and remote images are blocked by default; admins can allowlist image hosts.
- **SEC-14 (M)** — Rate limiting and brute-force protection on sign-in, the API, searches and MCP.
- **SEC-15 (M)** — A tamper-evident (hash-chained), append-only **security log** (audit log) records security events, permission changes, admin actions, and AI/MCP actions together with the human on whose behalf they ran.
- **SEC-16 (M)** — Secrets at rest (SMTP passwords, AI API keys, OIDC client secrets) are encrypted with an instance master key, which is generated on first run and kept outside the database.
- **SEC-17 (M)** — Search safety: searches compile to parameterized SQL through a query builder, never string concatenation. Searches have cost limits, timeouts and page-size caps.
- **SEC-18 (M)** — Supply chain:
  - lockfiles, `cargo-deny` / `cargo-audit` / `npm audit` in CI;
  - an SBOM (CycloneDX) per release;
  - signed release artifacts and checksums.
- **SEC-19 (V1)** — Security self-check page that flags weak configuration, such as HTTP exposed beyond localhost, no backups configured, admins without MFA, or AI sending data to an external provider.
- **SEC-20 (V1)** — Encrypted backups (see PRIV-10). Optional database-level encryption at rest is L.

## 2. Identity & authentication (AUTH)

- **AUTH-01 (M)** — Local accounts with Argon2id password hashing. The password policy follows NIST 800-63B: length over complexity, plus a check against a list of breached passwords.
- **AUTH-02 (M)** — **Passkeys / WebAuthn**, both as a second factor and for passwordless sign-in.
- **AUTH-03 (M)** — **Two-step sign-in** with a code app (TOTP) and **backup codes**. Admins can require two-step sign-in per role.
- **AUTH-04 (V1)** — SSO via **OIDC** (Entra ID, Google, Keycloak, Authentik …), with mapping from identity-provider groups to Bugaboo groups.
- **AUTH-05 (L)** — LDAP / Active Directory sync, and SAML.
- **AUTH-06 (M)** — Invitations (time-limited, single-use links). Self-registration is off by default; when enabled it can be restricted to approved e-mail domains.
- **AUTH-07 (M)** — **Access keys** (for scripts and apps) with scopes, expiry and per-project restriction. A key can only *narrow* its owner's permissions, never widen them.
- **AUTH-08 (V1)** — **Agents of the organization** for automation (webhooks, imports): agents (AI-30) whose responsible party is the organization, with a role (ADR-0029, decision 10).
- **AUTH-09 (M)** — First-run setup is protected by a one-time **setup code** printed to the server console, so nobody else on the network can claim a fresh instance.

## 3. Projects & hierarchy (PRJ)

- **PRJ-01 (M)** — Projects nested to any depth (organization → product → part …), each with a unique **short name** used in issue IDs (`WEB`, `WEBAPI`).
- **PRJ-02 (M)** — Settings are inherited down the tree, with per-project overrides. This covers permissions, workflows, issue types, fields, layouts and theme accent.
- **PRJ-03 (M)** — Move or re-parent projects. Permission changes caused by a move are shown before you confirm it.
- **PRJ-04 (V1)** — Archive projects (read-only, hidden by default) and restore them.
- **PRJ-05 (V1)** — Project templates: create a project from an existing configuration.
- **PRJ-06 (V1)** — Across projects: any search, board or timeline can cover a project and everything below it.

## 4. Issues / bug tracking (ISS)

- **ISS-01 (M)** — Issues with an **issue ID** (`WEB-142`), title, Markdown description, type, status, priority, *reported by*, *assigned to* (one or more people), and created/updated/closed dates.
- **ISS-01a (M)** — **Issue IDs, counting or random** (ADR-0021): each project chooses *counting* (`WEB-142`, easy to say aloud) or *random* (`WEB-K7Q2X`, five characters from an alphabet without look-alike letters). Random IDs don't reveal how many issues a project has or how fast it grows. New projects use the organization's default, which starts as **random**. Switching later keeps old IDs working.
- **ISS-02 (M)** — Configurable **issue types** (defaults: Bug, Task, Feature, Goal — all editable). Each type has its own icon, fields, layout and workflow.
- **ISS-03 (M)** — **Sub-issues**: any issue can sit under a parent (for example a Goal → Tasks → smaller Tasks). Progress, estimated time and time spent add up to the parent.
- **ISS-04 (M)** — **Related issues**: connect any two issues, also across projects.
  - Kinds of relation are editable. Each kind has a name for both directions (*blocks* / *is blocked by*, *duplicate of* / *has duplicate*, *causes* / *is caused by*, *related to*).
  - Add a relation from the issue page (*Add relation* → pick the kind → find the issue by number or title), by dragging on the timeline, or for many issues at once.
  - Both issues show the relation, and adding or removing it is recorded in both histories.
  - You only see a relation when you may see **both** issues.
- **ISS-05 (M)** — Comments with Markdown, @mentions and editing (edit history kept). Internal comments visible only to chosen roles are V1.
- **ISS-06 (M)** — A full activity/change history per issue, filtered by field permissions.
- **ISS-07 (M)** — **Follow** issues (and documents) to get notified; see who follows.
- **ISS-08 (V1)** — **Releases**: *found in release* and *planned for release* on issues, plus release status and dates. These are essential for bug tracking.
- **ISS-09 (M)** — Change many issues at once (fields, status, project), each with a preview and a permission check per issue.
- **ISS-10 (V1)** — Issue templates (for example, a bug report with steps-to-reproduce, expected and actual result).
- **ISS-11 (M)** — Draft autosave while creating or editing.
- **ISS-12 (V1)** — Clone issues, and move them between projects. Field mapping applies when the target project's configuration differs.
- **ISS-13 (M)** — Full-text search over titles, descriptions and comments. Results respect permissions.
- **ISS-14 (V1, last)** — Live updates: screens refresh when others change issues or documents. Implemented last in 1.0. Until then, edit conflicts are caught by optimistic locking, and the domain-event design leaves room for it.
- **ISS-15 (L)** — Reactions, issue voting.
- **ISS-17 (M)** — **Edit locks and confirmed saves** (ADR-0019):
  - Long text (descriptions, long-text fields, a comment being edited, document pages, admin editors such as workflows and themes) gets an **edit lock** on the first keystroke. Others see *"Ana is editing the description"*; the field is read-only for them until she's done.
  - The lock is a **lease** renewed by a heartbeat; a closed tab or lost connection frees it within a minute. With permission, someone can *Edit anyway*; the first person is told, and their unsaved text is kept.
  - **Drafts** are kept on the server and in the browser while typing, so no text is ever lost.
  - **Confirmed saves:** every change shows at once as *Saving…*, then *Saved* when the server confirms. If the save fails (someone changed it, the lock ran out, offline), the person sees why in plain words and keeps their text, with *Compare and save*.
  - Until live updates exist, open items are checked every 30 s: *"Ben changed the description after you."*
- **ISS-16 (V1)** — **Relation map**: an interactive map of everything connected to an issue, in any direction and through any number of steps.
  - Choose how far to look (1, 2, 3 steps or *everything*) and which kinds of relation to follow.
  - Pointing at an issue highlights the chain that connects it back to the starting issue. Clicking an issue moves it to the middle, so you can walk along a chain.
  - Circles in *blocks* chains (A blocks B blocks A) are flagged.
  - A list form shows the same chains as sentences (*"WEB-142 blocks WEB-147, which is blocked by WEB-148"*), for phones and screen readers.
  - Also available as **Show as → Map** for any search, which draws all results and the relations between them.
  - **Security:** only issues you may see appear. Chains never pass through hidden issues, so a map can't reveal that a hidden issue exists.
  - **Performance:** traversal runs on the server with a step and size limit (default: at most 200 issues), and sends only number, title, status and relations.

## 5. Custom fields (CF)

- **CF-01 (M)** — Field types:
  - text, long text (Markdown), integer, decimal, checkbox, URL;
  - date, datetime, duration;
  - single-select, multi-select, user, multi-user, group, version, issue reference.
- **CF-02 (V1)** — Cascading select. Computed/formula fields are L.
- **CF-03 (M)** — Fields have a scope (global, project subtree, issue type). They can be marked required and given a default value, validation rules (range, regex, length) and help text.
- **CF-04 (M)** — **Custom styles** per field and per option:
  - a color, an icon, and a display style (badge, pill, plain, progress bar, avatar);
  - all chosen from theme tokens, with guaranteed readable contrast.
- **CF-05 (V1)** — **Conditional styling rules**, for example "if Severity = Critical and Status ≠ Done, use the danger style".
- **CF-08 (M)** — **Own fields everywhere**: fields can be added not only to issues but also to time entries, working days, documents, releases and projects. They use the same field types, permissions, search and reports.
- **CF-06 (M)** — Every custom field has its own permissions (see SEC-04) and is fully usable in search, boards, the timeline and the API.
- **CF-07 (M)** — Fields are managed entirely in UI dialogs: create, edit, reorder options, merge options, and deprecate or archive fields without losing data.

## 6. Workflows (WF)

- **WF-01 (M)** — A workflow is a set of statuses and the **status changes** allowed between them. Every status belongs to a **status group** (*Not started / In progress / Done*). Workflows are assigned per project and issue type.
- **WF-02 (M)** — A **visual workflow editor** (drag statuses, draw arrows for allowed changes) and a plain list as an accessible alternative.
- **WF-03 (M)** — Rules on each status change: **Who can do it** (role, group, the person assigned, the person who reported it) and **Only if …** (field-based rules).
- **WF-04 (M)** — **Must be filled in** (for example, "Planned for release" before *Done*) and **Ask for** (a small form for fields during the status change).
- **WF-05 (V1)** — **Then automatically …**: fill a field, assign, notify, create a related issue, add a time entry.
- **WF-06 (M)** — Default workflows ship as editable templates: *Simple*, *Bug*, *Development* (with review/QA).
- **WF-07 (V1)** — Workflow versioning. When you publish a workflow that removes statuses, a step-by-step helper moves the affected issues to new statuses.
- **WF-08 (L)** — **Automations**: rules that run on a schedule or event (for example, "close Resolved issues after 14 days of inactivity").

## 7. Tags & labels (TAG)

- **TAG-01 (M)** — Free-form **tags** with autocomplete. Each tag has a color and can be restricted per project.
- **TAG-02 (M)** — Managed **labels**: an admin-curated set, optionally grouped (`area:auth`, `area:ui`). Labels can be exclusive within a group.
- **TAG-03 (M)** — Rename and merge tags or labels across all issues (audited).
- **TAG-04 (M)** — Tags and labels work everywhere: search, boards and timeline grouping.

## 8. Search (QRY)

- **QRY-01 (M)** — **Advanced search**: a readable *search language* typed as text. It uses the same field names as the UI (from the glossary), so what you see is what you type. For example:
  `project within WEB and statusGroup is not Done and assignedTo is me and changed in last 7 days sort by priority, highest first`
  The exact grammar is defined in the architecture phase; the goal is that people can read a search aloud.
- **QRY-02 (M)** — **Simple search**: filter buttons and dropdowns that convert exactly to and from Advanced search in both directions. Both show the same parsed search, so switching never loses information.
- **QRY-03 (M)** — One grammar for both client and server. The Rust parser is compiled to WebAssembly, so the editor's syntax highlighting, autocomplete and inline errors behave exactly like the server.
- **QRY-04 (M)** — Plain helper words such as `me`, `today`, `this week`, `in last 7 days`, `member of "Backend"`, `within WEB` (a project and everything below it), `blocks WEB-1`, `under WEB-7` (sub-issues), `connected to WEB-7 within 3 steps` (any chain of relations), plus text search (`contains "crash on save"`).
- **QRY-05 (M)** — **Security:** searches run as the calling user. Issue visibility, restricted issues and field permissions are added by the compiler, and a search that references an unreadable field is rejected with the same generic error as an unknown field.
- **QRY-06 (M)** — **Saved searches**: a search plus how it is shown (list, board, timeline, calendar). Private, or shared with groups or projects. Each one has a stable URL.
- **QRY-07 (V1)** — "E-mail me this search": scheduled e-mail summaries of a saved search.
- **QRY-08 (V1)** — Totals (count, sum of estimated time, grouped by a field) for dashboards and reports.
- **QRY-09 (V1)** — **Ask in your own words** (AI): the assistant turns a plain sentence into a search. The search is always shown, and can be edited, before it runs.

## 9. Time tracking (TIME)

- **TIME-01 (M)** — Start/stop **timer** on any issue. One active timer per user, synced across devices. The timer survives browser close and warns when it has been running for an unusually long time.
- **TIME-02 (M)** — Manual **time entries** with date, duration, description, an optional billable flag, and accounting fields (TIME-13).
- **TIME-03 (M)** — **Estimated time**, **time left** and **time spent**, rolled up through the issue and project hierarchies.
- **TIME-04 (M)** — A **weekly timesheet** with fast keyboard entry.
- **TIME-05 (V1)** — Reports by person, project, issue, tag, **activity**, **cost center**, any own field, or period, with CSV export.
- **TIME-06 (V1)** — Time entry permissions: who can see, change or delete their own time entries or other people's.
- **TIME-07 (V1)** — Period locking (a closed month can't be changed without a recorded reason). Approval workflow is L.

### Working hours (German working-time recording)

- **TIME-08 (M)** — **Working hours per day**, separate from time on issues:
  - *Start work day*, *Start break*, *End break*, *End work day*, from the sidebar, Home, the phone bottom bar or the timesheet;
  - or typed in afterwards (start, end, breaks) in the timesheet's **Work day** rows;
  - starting an issue timer while the work day hasn't started asks *"Start your work day too?"*;
  - shows hours worked today and this week, and the difference between working hours and time booked on issues.
- **TIME-09 (M)** — **Working-time rules** as a preset per country, editable by admins. The Germany preset follows the Working Hours Act (ArbZG): at most 10 hours a day, breaks of 30 minutes after 6 hours and 45 after 9, and 11 hours of rest between days. Breaking a rule shows a plain warning; it never blocks recording, because the record must show what really happened.
- **TIME-10 (M)** — **Trustworthy records**: changing a past work day requires a reason; the old value is kept in the history and the person is notified when someone else changes their record. Records are kept for the configured period (Germany preset: 2 years) and then deleted automatically (PRIV-05).
- **TIME-11 (V1)** — Export of working hours for payroll and HR (CSV; more formats L).
- **TIME-12 (M)** — **Who sees working hours**: everyone sees their own. Others need the separate permission *See working hours of others*, granted per group. An option makes team reports show totals only, for organizations with a works-council agreement.

### Accounting

- **TIME-13 (M)** — **Activity** and **Cost center** on every time entry, as ready-made own fields (CF-08) that admins can rename, change or remove like any other field:
  - *Activity* starts with *Development, Testing, Documentation, Meetings, Support*;
  - *Cost center* is a managed list of number + name (for example *4100 Product development*);
  - defaults come from the issue, then the project (inherited down the project tree), and can be changed per entry;
  - admins can make either field required, so every booked minute can be accounted for.

## 10. Planning & Gantt (PLAN)

- **PLAN-01 (M)** — **Timeline (Gantt chart)** for any search or project tree. Dates come from start/due fields (configurable which ones).
- **PLAN-02 (M)** — Dependencies: **must finish first**, plus *start together*, *finish together* and *finish when it starts* in V1. Dependencies are relation kinds that also affect dates, so there is one relation model for issues, the timeline and the map (SSOT).
- **PLAN-03 (M)** — Drag to move or resize bars, with undo and a permission check per issue.
- **PLAN-04 (M)** — Group rows by project, goal, assigned person, release or any choice field. Collapsible hierarchy, zoom from day to quarter, a today line, and milestones.
- **PLAN-05 (V1)** — **Deadline drivers** (critical path): highlight the chain of issues that sets the end date, and warn about impossible dependencies.
- **PLAN-06 (V1)** — **Board**: columns from status or any choice field, optional **rows** split by a field, **card limits** per column, and dragging a card changes its status through the workflow.
- **PLAN-07 (L)** — Baselines (planned vs. actual) and export to PNG/PDF. (The workload rows are the rows per person of PLAN-10.)
- **PLAN-09 (V1)** — **Calendar**: issues on a month or week calendar by due date. Drag an issue to another day to change its due date. Phones show a day-by-day list.
- **PLAN-08 (M)** — Restricted issues are left out entirely; there are no "ghost" bars or dependency arrows that reveal a hidden issue exists.
- **PLAN-10 (M)** — **The person on the bar**: each bar shows the picture and first name of the person it is assigned to; *Group by person* gives one row per person. Both follow *Who sees people* (ADR-0030).
- **PLAN-11 (M)** — **Away times** per person (whole or half days), entered by the person or, with *Enter work times for others*, by someone else; **no reason is stored**, others see only *Away*. **Usual work days** per person and **public holidays** per region (Germany per federal state). A new kind of people information *Work times*; away times are deleted a year after they end.
- **PLAN-12 (M)** — **Plan by effort**: an issue with an estimate and a person gets as many of that person's work days as the estimate times its time factor needs, each day counting with the person's work hours times their share in the project (PLAN-15), skipping away days, days off and public holidays; away days show as a hatched gap in the bar. Fixed due dates never move by themselves.
- **PLAN-13 (M)** — **Plan warnings** on the bar and in a list: *Away*, *Not enough days*, *Delay* (along *must finish first*), *Too much at once*; each with *Move it after*, *Make it longer*, *Give it to someone else…* (no suggestion of whom) or *Keep and accept*.
- **PLAN-14 (M)** — **Time factor** on an issue for its assigned person (*×1.5*): the person's own estimate, which **the assigned person may always set** on their own issues (also as *"Takes me 10 days"*); whoever plans may set it too, and the issue shows who did. It belongs to the issue and goes back to ×1 when the issue goes to another person. No factor per person, no defaults, no reports or sorting by it, never set by AI (PRIV-09, COMP-10).
- **PLAN-15 (M)** — **Work hours** per usual work day per person (from the work contract; at the start 8) and a **share in the project** per person (set by a Project lead on *Project members*; at the start 100 %); plan by effort and *Too much at once* use them. Shares over 100 % in total are shown to the person and their project leads.

## 11. Attachments (ATT)

- **ATT-01 (M)** — Upload by drag-and-drop, paste (screenshots) or the file picker, on issues and comments. Uploads resume after interruption.
- **ATT-02 (M)** — Safe storage and serving:
  - files are stored outside any web root and addressed by content hash;
  - downloads use `Content-Disposition: attachment` and `nosniff`;
  - SVG and HTML are never rendered inline.
- **ATT-03 (M)** — Size and type limits (configurable per project). The real file type is checked from its content, not trusted from the client.
- **ATT-04 (M)** — Image and PDF previews, generated on the server and served as safe derived images.
- **ATT-05 (V1)** — Optional malware-scanning hook (ClamAV), with files quarantined until scanned.
- **ATT-06 (V1)** — Pluggable storage: local disk (default) or S3-compatible.
- **ATT-07 (M)** — Attachment access follows the parent issue's permissions on every download, with short-lived signed URLs.

## 12. Configurable UI (CFG)

- **CFG-01 (M)** — **Layout editor**: drag fields into sections and tabs of the issue page, the new-issue form and the board card, per project and issue type.
  - **Role-based:** for each field in a layout, choose which roles see it (for example a short new-issue form for Guests). Field permissions always win: a role that may not see or change a field never gets it, whatever the layout says.
  - *Add a field* on an issue lists only fields the person's role may fill in.
- **CFG-02 (M)** — Configurable lists: columns, order, widths, density, grouping and sorting. All of it is stored in the **saved search**.
- **CFG-03 (V1)** — **Dashboards** built from boxes (widgets): search results, charts, time summaries, Gantt snippet, my timer, and recent activity.
- **CFG-04 (M)** — Personal preferences: theme, density, start page, date/time/number formats, week start, time zone, keyboard shortcuts.
- **CFG-05 (V1)** — Customizable navigation: pin projects, saved searches and documents to the sidebar.
- **CFG-06 (M)** — Settings apply in layers: **instance default → project → user**, each with "reset to default". Admins can lock settings.
- **CFG-07 (V1)** — Export and import of configuration (workflows, fields, layouts, themes, roles) as versioned JSON, for backup and to move it between instances.
- **CFG-08 (M)** — **Quick search** (Ctrl/Cmd + K): find anything, go anywhere, run any action.

## 13. Theming (THM)

- **THM-01 (M)** — Four built-in themes:
  - **Daylight** (light, neutral) and **Paper** (light, warm, softer contrast);
  - **Dusk** (dark, dim grey) and **Midnight** (dark, near-black/OLED).
- **THM-02 (M)** — The **system default is used**: Bugaboo follows the OS light/dark preference automatically. Each user picks a preferred light theme and dark theme, or forces one.
- **THM-03 (M)** — A **theme editor** in the UI:
  - every design token (colors, typography, radius, spacing, shadows, density, focus ring) with live preview;
  - contrast checking against WCAG AA, which warns and blocks failing text/background pairs.
- **THM-04 (M)** — Custom themes at instance level (admin) or personal level. Duplicate, import or export them as JSON.
- **THM-05 (V1)** — A project accent color, and branding (logo, product name, favicon) set by admins.
- **THM-06 (M)** — Honors `prefers-contrast` and `prefers-reduced-motion`.

## 14. AI & MCP (AI)

### Platform

- **AI-01 (M)** — Provider abstraction:
  - **Anthropic Claude** is the default;
  - any **OpenAI-compatible** endpoint also works (covers Ollama, LM Studio, vLLM and Azure), so the instance can run fully local and private.
- **AI-02 (M)** — **Off by default.** Admins enable AI per instance and per project and see exactly which data may be sent to which provider.
- **AI-03 (M)** — **AI field exclusion**: any field, project or restricted issue group can be marked "never sent to AI", even when the user can read it.
- **AI-04 (M)** — The AI always acts **as the current user**, with that user's permissions. It never gets a service-level view of the data.
- **AI-05 (M)** — Prompt-injection safeguards:
  - issue content is treated as untrusted data;
  - every write proposed by the AI needs explicit user confirmation, shown as a diff;
  - AI output is rendered with no remote images or auto-loaded links (this blocks data exfiltration);
  - each tool call is audit-logged.
- **AI-06 (V1)** — Usage limits and cost budgets per user and per instance.

### Assistant features

- **AI-10 (M)** — Assistant side panel, available wherever you are and aware of the current issue, search or project.
- **AI-11 (V1)** — Ask in your own words (see QRY-09).
- **AI-12 (V1)** — Summarize an issue or a long comment thread, and catch up on what changed since your last visit.
- **AI-13 (V1)** — Turn rough text, logs or a pasted stack trace into a structured bug report (steps, expected and actual result, environment).
- **AI-14 (V1)** — **Suggestions** for new issues (type, labels, priority), shown as accept/reject buttons. The assistant never suggests *who* should work on an issue (COMP-10).
- **AI-15 (L)** — Duplicate detection using embeddings (stored locally, filtered by permissions).
- **AI-16 (L)** — Release notes drafted from a release or search, and time entry descriptions drafted from activity.

### MCP server

- **AI-20 (M)** — A built-in **MCP server** (streamable HTTP) so external agents (Claude Code, Claude Desktop, IDEs) can work with Bugaboo.
- **AI-21 (M)** — MCP tools:
  - read: `search_issues` (search language), `get_issue`, `list_projects`, `get_my_work`;
  - write: `create_issue`, `update_issue`, `change_status`, `add_comment`, `log_time`, `start/stop_timer`;
  - plus resources (issues, saved searches) and prompts.
- **AI-22 (M)** — MCP authentication via **OAuth 2.1 with PKCE**, following the MCP authorization spec, or a scoped access key. Read and write scopes are separate, and scopes can only narrow the user's permissions.
- **AI-23 (M)** — Every MCP call goes through the same policy engine. Results are filtered per field, and changes are attributed in the audit log as *"user via MCP client X"*.
- **AI-24 (V1)** — Per-client consent screen and a **Connected apps** page where users can revoke access.

### Agents (ADR-0029)

- **AI-30 (M)** — **Agents**: accounts that work on their own and are not people. Each has one responsible person and can do at most what that person can, narrowed by its work and limits; it stops when the person loses access. Agents have no role, are in no group and are never *Assigned to* an issue. One person may have several agents working at the same time. Signing in with an agent key or OAuth; the permission *Create agents that work for you*; administrators see and stop every agent.
- **AI-31 (M)** — **Hand over** one or several issues by ID to an agent, with a brief (taken from the issue, plus a note; the window says what is missing), what the agent may do in Bugaboo, a budget and an optional plan approval. The issue keeps its person and shows *worked on by* the agent; the hand-over ends at the review or when it runs out.
- **AI-32 (M)** — **Standing rules**: an agent may work on everything a saved search finds, by proposals (default), by changing only named fields (labels, tags, type, duplicate of, found in release), or by making issues in one named project. Never fields that hold people, never the status; runs out after 30 days (at most 90); every new match is written down.
- **AI-33 (M)** — **Agent work** on the issue page, apart from the comments: the brief, attempts with plan, steps, results (links, test reports) and the time and cost the agent reports; the states *Waiting for the agent, Agent working, Waiting for you, Ready for review, Agent gave up, Stopped*, shown on list rows and searchable.
- **AI-34 (M)** — **Questions and plans**: an agent asks instead of guessing; its work waits and its person gets a notification; the answer joins the brief. An optional plan approval before each attempt.
- **AI-35 (M)** — **Review**: *Accept* (the reviewer picks the next status), *Send back* with a note (a new attempt), *Throw away* (everything the attempts changed in Bugaboo is undone in one step). Per project, *Accept by itself when its checks pass* (off at the start).
- **AI-36 (M)** — **MCP tools for agents**: take a handed-over issue (all or nothing), report a step, ask, attach a result, give up, and *changes since*; `preview` on every write tool; each tool says whether it changes something; no edit-lock takeover; texts marked with who wrote them (person in the project, Guest, outside, agent).
- **AI-37 (M)** — **My agents**: all of a person's agents and their work in one place, *Waiting for you* first (questions, plans, reviews, rules about to run out); pause or stop one, or *Stop all my agents*; limits per agent (changes per minute, issues at once, attempts). On a phone, *Waiting for you* first.
- **AI-38 (M)** — **Runs**: every session of an AI app acting for a person and every attempt of an agent is one run, shown as its list of changes and undone in one step; text written through MCP is marked *Written by an AI app* or *Written by an agent*.

## 15. Notifications & integrations (NTF)

- **NTF-01 (M)** — In-app **notifications** (mentions, assignments, changes to things you follow).
- **NTF-02 (M)** — E-mail notifications (SMTP), configurable per user and per event. Content is filtered by **the recipient's** permissions.
- **NTF-03 (V1)** — Outgoing **webhooks**, HMAC-signed, filtered by event and search. Each webhook runs as a chosen agent of the organization (AUTH-08).
- **NTF-04 (V1)** — REST API with an **OpenAPI 3.1** spec and a generated TypeScript client (the UI uses it too).
- **NTF-05 (V1)** — CSV import/export, and full JSON export of a project.
- **NTF-06 (L)** — Importers for Jira, GitHub Issues, GitLab and YouTrack.
- **NTF-07 (L)** — Git integration: commits and pull requests that mention `KEY-123` are linked to the issue.
- **NTF-08 (L)** — Create issues and comments by e-mail.
- **NTF-09 (L)** — Web push notifications for the installed PWA.

## 16. Responsive, mobile & accessibility (UX)

- **UX-01 (M)** — Fully responsive layout:
  - desktop shows multiple panes;
  - tablet shows collapsible panes;
  - phone shows a single pane with bottom navigation.
- **UX-02 (M)** — Touch-friendly targets and gestures (swipe actions on list items). On phones the timeline is simplified to a list that you can scroll.
- **UX-03 (V1)** — Installable **PWA**, with read-only offline cache of recently viewed issues.
- **UX-04 (M)** — Accessibility to **WCAG 2.2 AA**: full keyboard operation, screen-reader labels, visible focus, and no information carried by color alone.
- **UX-05 (M)** — Keyboard-first power use: shortcuts throughout, with a `?` cheat sheet.
- **UX-06 (M)** — Optimistic updates and skeleton loading, with a perceived latency target under 100 ms for common actions.
- **UX-07 (M)** — English only, but ready for translation:
  - every user-facing string lives in one message catalog, and dates and numbers are formatted per locale;
  - **hooks reject hard-coded UI strings** (see coding-standards.md).

## 17. Installation & operations (OPS)

- **OPS-01 (M)** — A **single self-contained binary** per platform with the web UI embedded:
  - Windows x64/ARM64;
  - Linux x64/ARM64 (static, for any distro, including **WSL**).
- **OPS-02 (M)** — **Embedded database (SQLite) by default**, so the server works with zero setup. **PostgreSQL** is supported for larger or high-availability deployments (V1).
- **OPS-03 (M)** — `bugaboo setup` interactive wizard (data directory, port, TLS, admin bootstrap), then the remaining setup happens in the browser.
- **OPS-04 (M)** — Run as a service: `bugaboo service install` (Windows Service / systemd unit).
- **OPS-05 (V1)** — Packages: winget and MSI on Windows, `.deb`/`.rpm` on Linux, and an OCI/Docker image.
- **OPS-06 (M)** — Automatic schema migrations on upgrade, with an automatic backup before migrating and a clear rollback path.
- **OPS-07 (M)** — Online **backup and restore**, both from the CLI and the admin UI, with scheduled backups and retention.
- **OPS-08 (M)** — Health endpoint, structured logs (JSON), and an admin system-status page.
- **OPS-09 (V1)** — Prometheus metrics endpoint (off by default, authenticated).
- **OPS-10 (M)** — Bootstrap settings (bind address, port, data directory, TLS) live in a small config file or environment variables. *Everything else* is configured in the UI.
- **OPS-12 (V1)** — **Try it out**: the setup wizard can load the fictional sample organization (architecture §11), clearly marked as sample data and removable in one click.
- **OPS-11 (V1)** — In-app update notification with signed update verification (needed for COMP-04).

## 18. Document management (DOC)

Documents are a **first-class domain entity**, like issues. They ship in 1.0, but the data
model, permission engine, search, API, links and audit log are designed for both from day one,
not added later. Each project has its own **Documents** section. Documents use the same permissions,
storage, preview and search systems as issues and attachments, so nothing is built twice.

- **DOC-01 (V1)** — Documents per project:
  - folders nested to any depth;
  - **pages** (Markdown, rich editor);
  - **files** (any uploaded file, with previews from ATT-04).
- **DOC-02 (V1)** — **Versioning**: every save or upload creates a version. You can compare two versions and restore an old one, and each version records author, time and change note.
- **DOC-03 (V1)** — Permissions are inherited from the project, with overrides per folder and per document. Restricted documents use the same visibility groups as restricted issues (SEC-05). Viewing, editing, deleting and managing versions are separate permissions.
- **DOC-04 (V1)** — Links between documents and issues, in both directions. Issue IDs inside a page (`WEB-142`) render as live chips that show status and title, but only if the reader may see that issue.
- **DOC-05 (V1)** — Full-text search over pages and file contents (PDF and text in V1, Office formats in L), with results filtered by permissions.
- **DOC-06 (V1)** — Page templates (specification, runbook, meeting notes, release checklist), editable in the UI.
- **DOC-07 (V1)** — Pages use edit locks and confirmed saves (ISS-17), with a compare dialog when needed. Files can be locked for editing while someone works on them outside Bugaboo.
- **DOC-08 (V1)** — Trash with restore, and retention rules per project.
- **DOC-09 (V1)** — AI and MCP: the assistant can answer questions over documents (retrieval filtered by permissions), and MCP gets `search_documents`, `get_document` and `create_page`.
- **DOC-10 (L)** — Real-time collaborative editing, approval/review workflow for documents, and documents in search.

## 19. Performance & lean payloads (PERF)

- **PERF-01 (M)** — Server response targets at 1.0 scale (~500 users, ~1 M issues):
  - p95 under 100 ms for issue pages and saved searches;
  - p95 under 300 ms for complex searches and timeline data;
  - measured by an automated benchmark against a generated dataset of that size.
- **PERF-02 (M)** — **Lean responses**:
  - every list endpoint returns only the fields the view asks for, and only the fields the user may read;
  - cursor pagination with capped page sizes;
  - no embedded objects unless requested.
- **PERF-03 (M)** — Compression (Brotli and gzip; JSON above 1 KiB, never an answer that holds a secret) and caching validators (ETag / 304) for API responses. Static assets are content-hashed and cached forever.
- **PERF-04 (M)** — Frontend budgets, enforced in CI:
  - initial route JavaScript ≤ 250 KB (Brotli);
  - Gantt, workflow editor, theme editor, document editor and the search-language WebAssembly are loaded only when first opened;
  - lists are virtualized.
- **PERF-05 (M)** — No N+1 queries: query counts are asserted in tests for every list endpoint.
- **PERF-06 (V1, last)** — Live updates (ISS-14) send small change events, not full objects. Clients fetch details only for what is on screen.

## 20. Privacy & GDPR (PRIV)

Bugaboo is built so an organization can run it in line with the GDPR. Bugaboo supports compliance; the organization running it remains responsible as the controller.

- **PRIV-01 (M)** — **Privacy by design and by default**: features collect only what they need. No tracking, analytics, third-party scripts, outside fonts or CDNs. Only essential cookies, so no cookie banner is needed.
- **PRIV-02 (M)** — **Personal-data map**: every stored field that holds personal data is marked in the data model (single source). Export, erasure, retention and the processing record are all generated from these marks, so a new feature can't forget them.
- **PRIV-03 (M)** — **Download my data** (access and portability): every person can download everything Bugaboo stores about them, as readable pages and as machine-readable JSON. Admins can create the same export for any person; doing so is recorded in the security log.
- **PRIV-04 (M)** — **Remove a person** (erasure): their account is deleted and their name is replaced everywhere by *"Former user K7Q2"*. The organization chooses whether text they wrote stays (anonymized) or is removed. Records that must be kept by law (working hours, security log) stay until their retention period ends; the screen explains this in plain words.
- **PRIV-05 (M)** — **Keep and delete rules** per kind of data (working hours, time entries, security log, notifications, sign-in history, IP addresses, trash, backups), with automatic deletion. Sensible defaults, for example IP addresses in logs after 7 days.
- **PRIV-06 (M)** — People can correct their own profile data; admins can correct anything else, with a record of the change.
- **PRIV-07 (V1)** — **Record of processing** (Art. 30 GDPR) and a privacy notice template, generated from the personal-data map and the enabled features.
- **PRIV-08 (M)** — **Outside services**: the assistant, e-mail and any external provider are off until an admin enables them. Before enabling an external AI provider, the admin confirms that a data processing agreement is in place, and sees which data goes where (provider and country). Local models are offered as the private choice.
- **PRIV-09 (M)** — **Protecting employee data**: working hours and time entries are treated as sensitive employee data. Tight visibility (TIME-12), no performance scores, no ranking of people, and no AI analysis of anyone's working time or performance (a hard rule, COMP-10). Changes to these settings are recorded.
- **PRIV-10 (M)** — **Security of processing**: TLS everywhere, encrypted secrets, encrypted backups (moved from SEC-20 to V1), and a record whenever an admin views another person's data through *See as* or an export.
- **PRIV-12 (M)** — **Who sees people** (per role, per project, inherited down the tree): for each kind of people information — *reported by*, *assigned to*, *last changed by*, comment and history authors, followers, *"… is editing"*, time entries, mentions, avatars — choose **Names**, **Role only** (*"Someone (Team member)"*) or **Hidden** (*"Someone"*). Applies everywhere: screens, search, exports, e-mails, the assistant, MCP and the relation map. People who can't see names can't search or filter by person either (same rule as hidden fields, SEC-06).
- **PRIV-13 (M)** — **No counts that leak**: readable IDs can be random (ISS-01a), removed people get random labels (*Former user K7Q2*), and screens never show totals that include things the viewer can't see.
- **PRIV-11 (V1)** — **Data breach help**: the security log and the personal-data map show which data and which people a security incident could affect.

## 21. EU law & product documentation (COMP)

Bugaboo is built and documented so it can be **sold** (self-hosted by the customer) and
**served** (hosted by us in the EU), and meet EU law in both cases. Each item is tagged with
the model it matters for. The plain-language explanation of each law, who must do what, and
the open points for the legal review are in [compliance.md](compliance.md). Not legal advice;
a lawyer reviews this before the first release.

### Cyber Resilience Act (CRA)

- **COMP-01 (M) [sold, served]** — **SBOM for every release**, in CycloneDX and SPDX, covering Rust crates, npm packages, bundled fonts and icons. Generated automatically from the lockfiles (single source), signed and published with the release (extends SEC-18).
- **COMP-02 (M) [sold, served]** — **Third-party components register** (`docs/third-party/`): every direct dependency with purpose, license, maintainer health and alternatives considered. CI fails when a direct dependency has no entry or when an entry has no dependency, so the register can't drift. A **third-party notices** file with all license texts is generated for each release and shipped inside the binary (*About › Licenses*).
- **COMP-03 (M) [sold, served]** — **Vulnerability handling**: a published security policy and `security.txt`, coordinated disclosure, security advisories, VEX statements that say whether a vulnerable dependency actually affects Bugaboo, and reporting of actively exploited vulnerabilities and severe incidents within the CRA deadlines (24 h / 72 h / final report; runbook in DIST-08). The CRA reporting duty already applies (since 11 September 2026).
- **COMP-04 (M) [sold, served]** — **Secure by default and security updates**: the CRA's essential requirements map to existing features (SEC, AUTH, OPS-06/07). A declared **support period** per release (normally at least 5 years), free security updates during it, and security fixes shipped separately from feature updates where possible (OPS-11, DIST-07).
- **COMP-05 (V1) [sold]** — **Technical documentation file** per release: architecture, risk assessment, SBOM, test reports, support period, EU declaration of conformity, CE marking information, and instructions for secure installation and operation, assembled automatically from the repository.

### Architecture & documentation

- **COMP-06 (M) [sold, served]** — **Architecture documentation** kept with the code, in a fixed structure (arc42 chapters, C4 diagrams as text, decision records). It is part of the technical documentation (COMP-05) and checked in CI: a change to a module boundary, an external interface or a dependency needs a matching doc or decision-record change.
- **COMP-07 (M) [sold, served]** — **Security and data-flow documentation**: threat model, trust boundaries, and a data-flow map of personal data (generated from the personal-data map, PRIV-02).

### AI Act

- **COMP-08 (M) [sold, served]** — **People know when AI is involved**: the assistant is always labelled as AI, and everything it writes or proposes is marked *"Written with the assistant"* (AI-05, AI-10). Admins get information about the AI model in use before switching it on (PRIV-08).
- **COMP-09 (M) [sold, served]** — **Machine-readable marking**: text the assistant wrote (or that was accepted from it) carries a stored flag *written with the assistant*, shown in the UI, kept in the history and included in every export and in the API.
- **COMP-10 (M) [sold, served]** — **No AI decisions about people** (hard product rules, checked in review):
  - no AI that suggests who should work on something, assigns tasks, or ranks people;
  - no AI that monitors or evaluates people's performance or behaviour, including their working hours;
  - no emotion, mood or sentiment analysis of people's comments or messages.
  These would make Bugaboo a high-risk or even banned AI system at work. See also PRIV-09.
- **COMP-11 (M) [sold, served]** — **AI literacy guide**: a short guide for admins and users (what the assistant can and can't do, where data goes, how to check its output), linked from the assistant and from the setup screen, so customers can meet their own AI literacy duty.

### Other EU rules

- **COMP-12 (V1) [served]** — **Data Act switching**: complete export of all data and configuration in open, documented formats (extends NTF-05 and CFG-07), import on the other side, no switching charges, and published information on data location (see DIST-05, DIST-06). For the sold model this is good practice rather than a duty.
- **COMP-13 (M) [sold, served]** — **ePrivacy**: only strictly necessary cookies and storage, so no consent banner is needed (PRIV-01).
- **COMP-14 (V1) [sold, served]** — **NIS2 support for customers**: the security log, incident timelines and exports help organizations that fall under NIS2 meet their own reporting duties, and a security information pack (SBOM, vulnerability handling, security features) answers their supplier checks.
- **COMP-15 (M) [sold, served]** — **Accessibility**: WCAG 2.2 AA (UX-04) and the European standard EN 301 549, with an accessibility statement template.
- **COMP-16 (V1) [sold, served]** — **Product liability**: release notes, known issues and the support period are published with every release, and old versions stay downloadable with their SBOM.

## 22. Sold and served (DIST)

Bugaboo may be sold, served, or both. The decision is open, so both are designed in from
the start. See [compliance.md](compliance.md) for what each model means legally.

- **DIST-01 (M)** — **One codebase and one server binary for both models.** No separate "cloud edition"; behaviour differences come from configuration only.
- **DIST-02 (V1)** — **Served = one isolated instance per customer**: own database, files, encryption keys and web address, in EU data centres. A separate **control plane** handles set-up, updates, backups and monitoring. No database is shared between customers. This keeps the "one organization per instance" decision.
- **DIST-03 (M)** — **Operator access**: our staff have no access to customer data by default. Emergency access only with the customer's approval, time-limited, and recorded in the customer's own security log.
- **DIST-04 (V1)** — **Sold model licensing**: a signed license file that works offline. No phone-home, no telemetry.
- **DIST-05 (V1)** — **Served model paperwork**: data processing agreement, sub-processor list, technical and organisational measures, data location, exit terms, service levels, and a status page.
- **DIST-06 (V1)** — **Switching**: full export and import between served and self-hosted, in both directions (Data Act, COMP-12).
- **DIST-07 (M)** — **Updates**: self-hosted installations get signed updates and update notifications (OPS-11); served instances are updated by us, with announced maintenance windows and release notes.
- **DIST-08 (V1)** — **Incident reporting runbook** for us as manufacturer and operator, with the CRA and NIS2 deadlines (24 h early warning, 72 h notification, final report) and ready-to-fill templates.

---

## Non-goals for 1.0

- Several organizations sharing one instance or one database. The served model gives each customer their own instance instead (DIST-02).
- Native mobile apps (the responsive web app and PWA cover handhelds).
- Chat, CI/CD pipelines.
- AI that evaluates, ranks or assigns work to people, and any emotion or mood analysis (COMP-10).
- A plugin system that runs third-party code inside the server. Webhooks, the API and MCP are the integration points instead; this keeps the security boundary intact.

## Decisions

| Topic        | Decision                                                                 |
|--------------|--------------------------------------------------------------------------|
| Database     | SQLite embedded by default; PostgreSQL supported (OPS-02)                |
| Tenancy      | One organization per instance                                            |
| Additions    | Kept: restricted issues (SEC-05), releases (ISS-08), board (PLAN-06), notifications & integrations (NTF) |
| Naming       | Plain language for users without technical background; no terms copied from other vendors. [Glossary](glossary.md) is the single source |
| License      | Decide later. The repo stays private; dependency licenses are checked in CI |
| Scale (1.0)  | ~500 users, ~1 M issues per instance                                     |
| UI language  | English only. All strings in one catalog; hooks block hard-coded strings |
| Documents    | First-class entity, built into the core design from the start; DOC features ship in 1.0 |
| Live updates | Kept for 1.0, implemented last (ISS-14, PERF-06)                         |
| Editing      | No real-time co-editing in 1.0; edit locks with heartbeats, kept drafts and confirmed saves instead (ISS-17) |
| Proposals    | UX design and architecture accepted                                      |
| Testing      | Sample organization (4 projects, 2 with sub-projects, ≈ 10,000 issues, 50 people), 10-person concurrency tests, Vite + Vitest + Playwright on ports 47200–48099, a block of 100 for each work folder (architecture §11) |
| People and IDs | Who sees people is configurable per role (PRIV-12); issue IDs counting or random, random by default (ISS-01a) |
| Engineering  | SSOT, DRY, ODR mandatory; enforced by hooks ([coding-standards.md](coding-standards.md)) |
| Calendar     | In 1.0 (PLAN-09)                                                         |
| Relations    | Relations editable on every issue (ISS-04); relation map in 1.0 (ISS-16) |
| Look         | Violet highlight color. Fonts: Atkinson Hyperlegible Next and Atkinson Hyperlegible Mono |
| Working hours | Daily start, end and breaks recorded (German working-time recording), first release (TIME-08…12) |
| Accounting   | Activity and cost center on every time entry, fully customizable (TIME-13, CF-08) |
| GDPR         | Required everywhere, first release (PRIV section)                        |
| EU law       | Built to meet CRA, AI Act, NIS2, Data Act, GDPR, ePrivacy, accessibility and product liability rules; SBOM, third-party register and architecture docs are release deliverables (COMP section, [compliance.md](compliance.md)) |
| Distribution | Sold (self-hosted) and served (hosted by us in the EU), both supported from one codebase (DIST section) |
| AI and people | No AI decisions about people: no assigning, ranking, evaluating or mood analysis (COMP-10) |

## Open questions

1. **Legal review:** which EU laws apply, and how, depends on the license and on whether Bugaboo is sold, served or both. Plan a review by a lawyer before the first public release; the open points are listed in [compliance.md](compliance.md).
2. **Company size:** decides whether NIS2 applies to us as a hosting provider (roughly 50+ employees or more than €10 million turnover). Settle once the business model is known.
