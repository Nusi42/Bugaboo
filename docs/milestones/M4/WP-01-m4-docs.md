# WP-01 — Words, permissions, decision records, the name map and the maintainer's answers

**Wave 1 · Folder / branch:** `bugaboo.win1` / `WIN1` (lead)
**Features:** all of M4, the docs they need (WF-01…WF-06, ATT-01…ATT-07, NTF-01, NTF-02); ADR-0008, ADR-0031, ADR-0032, ADR-0033.

## Goal

Before code names anything new, the words, the decisions and the maintainer's answers are written down once, so the builders of waves 1 and 2 read one source.

## Done by the lead

1. **Answers:** all proposals of M4-Q1…Q10 accepted. Where each one is written:
   - **M4-Q1** (starter workflows): ADR-0031 §5 and the glossary (*Starter workflows*).
   - **M4-Q2** (*Manage workflows*): ADR-0031 §2 and §4. The permission itself is added by WP-04, after WP-02 renames the permission words.
   - **M4-Q3, Q4** (files): ADR-0032 §2.
   - **M4-Q5, Q6, Q7** (notifications and e-mail): ADR-0033 §1, §3, §4, UX §4.11.
   - **M4-Q8** (the API freeze): ADR-0008 amendment M4 and the planned check ARCH-API-FREEZE.
   - **M4-Q9** (handles): ADR-0033 §6 and the glossary (*Handle*).
   - **M4-Q10** (confirming an address): ADR-0033 §3 and the glossary.
2. **The name map** for WP-02, in [WP-02](WP-02-names-before-the-api-freeze.md), and its glossary rows:
   - *Create*;
   - *Run* (also of a change to many issues);
   - *Label group*;
   - *Combine* (retire, not remove);
   - *Layout* (issue page);
   - the internal terms *Readable ID* and *Edit notice*.
3. **Decision records:** ADR-0031 *Workflows*, ADR-0032 *Files*, ADR-0033 *Notifications and e-mail*; the ADR-0008 amendment.
4. **Glossary:**
   - workflows: *Starter workflows*, *Publish*, *Try it*;
   - files: *Upload*, *Files*, *Preview* (of a file), *Download*;
   - notifications and e-mail: *Mark as read*, *Customize*, *Mail server*, *Short e-mails*, *Handle*, *Confirm your e-mail address*;
   - internal terms: *Signed link*, *Notifications reader*.
5. **Checks** (planned, in the catalogue): TEST-WORKFLOWS, TEST-UPLOADS, ARCH-API-FREEZE.
6. **Security chapter:** the section *Workflows, files and notifications (M4)*.
7. **UX design:** the *Files* block (§4.4) and the notification settings, filters and e-mails (§4.11).

## Decided in WP-01 for the follow-ups the plan left open

- **M3-F23:** *Priority* stays a fixed list of four. An own *Pick one* field covers other needs.
- **M3-F100:** the issue page gets no *Add a field* menu in M4. Empty fields already show as a quiet *"Add due date"*.
- **M1-F10, M2-F03:** done in M3 WP-02 (the WebAssembly budget is in `tools/checks/perf-bundle.toml`). Marked done in their tables.

## Still to do by the lead before wave 2

- Mark M1-F10 and M2-F03 done.
- Note in M1-F30 that M4 WP-05 takes it.
