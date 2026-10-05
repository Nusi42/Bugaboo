# WP-01 — Permission model, words and M2 design docs (lead)

**Wave 1 · Folder / branch:** `bugaboo.win1` / `WIN1`
**Features:** design for SEC-01, SEC-02, SEC-03, SEC-04, SEC-06, SEC-08, PRIV-12, PRIV-13, AUTH-07, PRJ-01, PRJ-03; COMP-07 (threat model grows); ADR-0004, ADR-0018, ADR-0021, ADR-0022.

## Goal

Everything wave 2 builds on is written down before code: the permission model in detail, the words people will read, the threat model for permissions and access keys, and the module map for the sample-organization generator. The maintainer's answers to M2-Q1 … M2-Q7 go in here.

## Owns

`docs/adr/0025-*.md` (new), `docs/architecture/04-building-blocks.md` (module map), `docs/architecture/07-cross-cutting.md` (section *Authorization model* only; WP-02 owns *Performance*), `docs/architecture/12-security.md` (M2 section), `docs/glossary.md`, `docs/02-ux-design.html` (§5.1, §5.2 where an answer changes them), `docs/milestones/M1/follow-ups.md` (row M1-F49), `docs/STATUS.md`.

## Tasks

1. **ADR-0025 — Roles, groups and assignments on the project tree** (refines ADR-0004, does not replace it):
   - the **permission catalogue**: one Rust enum in `bb-policy`, grouped by topic (*Projects, People, Settings, Issues, Documents, Time*), each permission with a plain sentence as a message key (UX §5.2: "Change issues other people created"); the M2 list (see projects, make projects and sub-projects, move projects, change project settings, manage who has which role, manage people, manage groups, manage roles, check access, change organization settings, see the status page, manage invitations) and how later milestones add theirs;
   - **scopes** of an assignment: the whole organization, one project only, or a project and its sub-projects (SEC-02); a person or a group is assigned a role;
   - the **deny rule** from M2-Q1 and the order of evaluation (architecture §7, steps 1–4, made exact);
   - **starting roles** (SEC-08): one Rust definition of their starting permissions and default *Who sees people* (M2-Q5), used once to make editable copies at the first start, and for *Reset to the starting permissions*; never read again after that;
   - the **administrator flag** of M1 becomes the Administrator role on the whole organization (migration), and a change that would leave nobody able to manage roles on the whole organization is refused;
   - **two-step sign-in per role** (M1-F45): a switch stored with the role replaces the text setting `security.two-step-required-roles`, which is removed;
   - **access keys** (AUTH-07): scopes and projects only narrow (intersection with the owner's permissions, evaluated at every use), the choices of M2-Q6;
   - every change of roles, groups, members, assignments, field permissions, *Who sees people* and project moves is a domain event **and** a security-log entry (SEC-15);
   - evaluations are cached per request, never across requests.
2. **Architecture §7, Authorization model:** add the **field catalogue** (built-in fields from M2-Q7: key, message key, which entity, whether it holds a person) declared once in `bb-core`; the **kinds of people information** (PRIV-12, plus *Project members* if M2-Q3 says so) with their defaults; how `PersonRef` resolves (viewer, kind, project → name, role label, *Someone*, or a random *Former user* label for a removed person, PRIV-13); the shape of a **Check access** answer (a list of reasons — person, group, role, project, inherited from — that the screen turns into sentences with message keys).
3. **Personal data:** group membership and role assignments are personal data (category and basis decided here, perhaps a new category *Organization*: where a person works); access keys (name, last use); the person settings of *My settings* (time zone is personal). Add them to the data-flow part of `12-security.md`.
4. **Threat model, M2 section** (`12-security.md`): scope table gains the permission engine and access keys; STRIDE rows for privilege escalation by editing a role one holds, reaching hidden projects by address or short name (a taken short name tells only people who may make projects that it exists; the answer never says whether the project is hidden), counts that leak, a stolen access key, keys that outlive their owner's permissions; each row names the package test or check that will prove it (SEC-MATRIX, ARCH-AUTHZ-ONLY, ARCH-PERSON-REF, ARCH-NO-COUNTERS, named tests).
5. **Glossary:** the words M2 shows that are not there yet, before anybody names them: the deny word (M2-Q1), the three scopes (*Whole organization*, *Only this project*, *This project and its sub-projects*), *Starting role*, *Reset to default*, *Set by your organization* (a locked setting), *Project members* (if M2-Q3), the access-key choices (M2-Q6), *Density* or the word the UX design uses (*Spacing*), *Less motion*, *Larger text*, *Keyboard shortcuts*. Each with its *Don't say* column (for TXT-TERMS, WP-04).
6. **Module map** (`04-building-blocks.md`, META-NEW-MODULE): a new crate `bb-sample` (the sample-organization generator; library plus a small program that xtask runs, so xtask keeps depending on no product crate; depends on `bb-app`, `bb-store`, `bb-core`, `bb-policy`, `bb-auth`; not linked into `bugaboo` until OPS-12) and its ARCH-DEPS rule; the new browser feature folders (`settings`, `people`, `roles`, `projects`, `my-settings`).
7. **UX design page:** map the topics of the settings registry to the *Organization settings* menu (§5.1: where `security.*` and `auth.*` settings appear), and write the answers of M2-Q1 and M2-Q3 into §5.2; republish the page for the maintainer.
8. **STATUS and follow-ups:** M2 waves in `docs/STATUS.md`; move M1-F49 to M4 (with M1-F53).

## Acceptance

- META-RULE-TRACE, DOC-ADR-ENFORCED, FMT-MD, LINT-SPELL and LINT-LINKS green; every new rule names its check or says *review* with a reason.
- The answers to M2-Q1 … M2-Q7 are in the ADR, the glossary or the UX page, and the README's question list says where.
- A builder can start WP-05 from the ADR and architecture §7 alone, without asking.

## Depends on

M1 merged; the maintainer's answers (M2-Q1 … M2-Q7) before this package is merged.
