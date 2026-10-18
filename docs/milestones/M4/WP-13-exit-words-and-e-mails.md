# WP-13 — Exit review: words, e-mails and user docs

**Exit wave A · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** WF-01…WF-06, ATT-01…ATT-07, NTF-01, NTF-02 (their texts, e-mails and user docs).
**Source:** [the M4 exit review](exit-review.md), section *Words review*.

## Goal

Every text of M4 says what Bugaboo really does, in the glossary's words, the same way on the screen, in the e-mail and in the user docs.

## Owns

`web/packages/messages` (texts only; keys that WP-14 adds for its screens are WP-14's), `crates/bb-app/src/notifications/compose.rs` (the sentences of e-mails), `userDocs/**`, `docs/glossary.md`, the text parts of `docs/02-ux-design*` (UX §5.1, §5.4).

## Tasks

1. **A1–A11** (texts that lie): e-mail sentences reuse the screen's history sentences and value wording (one source for "changed", "set", "set people", "cleared", the long-text sentence, yes/no, times and days in the reader's region); the released-lock sentence; the issue-ID renewal notice gets its own sentence and help text; the stale user docs; the UX doc and glossary leftovers. A1 (files): WP-12 makes adding a file notify; keep the "file" texts and add the sentence "{who} added a file".
2. **B1–B14**: one name for each thing. Decided by the lead: *Reset to default* for taking a place's own workflow away (hidden at the organization level, see WP-14); *link*, never *address*, for something you open; *label group*, never a bare *group*, for labels; *offered* / *does not offer*; *Test e-mail* for the mail server test; *Upload files* also on comments; *computer*, never *machine*; no *lock* in UI text; one *Your change to many issues has ended*; the starter workflow; "Send me a new link" for both resend buttons. Texts written twice become one key (B14).
3. **C1–C18**: the plainer texts as proposed, unless a proposal would contradict the glossary.
4. **Glossary**: the rows to add or change at the end of the words section.
5. The words-related lead notes: one sentence form for every field in *History* ("set Reported by: …" has a colon where others say "to").

## Checks

TXT-TERMS, TXT-LITERAL. If the e-mail sentence reuse needs a generated table shared by Rust and the browser, generate it (one source).

## Depends on

`main` after WP-10. Works beside WP-12 (code of notifications) and WP-14 (screens).
