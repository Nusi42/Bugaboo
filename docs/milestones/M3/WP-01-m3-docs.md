# WP-01 — Words, permissions, decision records and the maintainer's answers

**Wave 1 · Folder / branch:** `bugaboo.win1` / `WIN1` (lead)
**Features:** all of M3 (the docs they need); ADR-0006, ADR-0007, ADR-0011, ADR-0019, ADR-0021, ADR-0025.

## Goal

Before code names anything, the words, the permissions, the storage of layouts and per-type rules and the maintainer's answers are written down once, so the builders of waves 1 and 2 read one source.

## Owns

`docs/glossary.md` (the M3 words), `crates/bb-policy/src/catalogue.rs` and `starting.rs` (the M3 permissions with their starting values; the generated table in architecture §7 by `cargo xtask gen roles`), `docs/adr/0019-*.md` and `0021-*.md` (amendments), `docs/adr/0027-layouts-and-rules-per-issue-type.md` (new), `docs/architecture/07-cross-cutting.md` (*The field catalogue*, *Authorization model*), `docs/architecture/12-security.md` (M3 section), `docs/02-ux-design.html` (§4.4 History, §5.1 menu rows for the new setting keys, §5.4 *Labels & tags*), `docs/04-milestones.md` (M3 section), the follow-up tables.

## Tasks

1. **Answers:** write each answer of M3-Q1…Q10 where it belongs and add "Answered" lines to the plan, like M2.
2. **Glossary first:** *Comment*, *Mention*, *Draft*, *History* (tabs *Everything · Comments · Changes*), *Choice*, *Combine with another choice*, *Retire*, *Look*, *Pick one group*, *Change many issues*, *Ask … to finish*, *Keep Ben's / Use mine*, *Show changes*, *Add a field*, *Starting workflow*, *Issues* and *Work settings* (permission topics), each with its "don't say" words.
3. **Permissions:** add the M3 permissions with topic, sentence, kind, starting values and `NOT_FOR_KEYS` as answered in M3-Q6. Each needs a route before ARCH-PERMS passes: add them to `tools/checks/permission-exemptions.toml` with "route in WP-nn" and the package removes the line.
4. **ADR-0019 amendment:** the version travels in the body (`version`), one per item for many changes; the short-value conflict answer is structured (who, when, which field, their value) so the browser writes the banner of M3-Q5 (d) with the reader's mask and *Who sees people*; error messages keep no placeholder. Drafts: keep period of M3-Q9.
5. **ADR-0021 amendment:** a move to another project gives a new ID and an alias (M3-Q4).
6. **ADR-0027 (new):** layouts per project and issue type, field levels per issue type, kinds of relation; storage in own tables; the lookup order (nearest project, then organization; at one place a row for the type before a row for all types); the starting workflow of M3-Q1 and how M4 replaces it.
7. **Architecture §7:** the field catalogue becomes `FieldKey` (built-in or own); own fields' starting levels (the field's creator picks, default *Can see* for everyone who sees the project, *Can change* for Team member and up).
8. **Security chapter (§12):** M3 threats: stored rich text (SEC-13), mentions and autocomplete leaking names or hidden projects, relations that show a hidden issue, lock notices that show a hidden name, drafts as personal data, many changes at once bypassing a per-issue check.
9. **UX:** History tabs; the *Labels & tags* settings page (list → side panel: name, color token `cat.1…12`, group, *Pick one* group, projects, *Combine*, *Retire*); the long-preview pattern of M2C-F80 (summary first, grouped by issue or by person, "Show all 120").

## Acceptance tests (write first)

- `cargo xtask gen roles` reproduces the starting-roles table; `sec_02_starting_roles_have_the_m3_permissions` (unit test in `bb-policy`).
- META-RULE-TRACE, TXT-TERMS and FMT-MD green on every changed doc.

## Checks

None new.

## Depends on

The maintainer's answers to M3-Q1…Q10.
