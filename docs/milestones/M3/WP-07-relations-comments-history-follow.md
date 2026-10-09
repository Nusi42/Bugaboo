# WP-07 — Sub-issues, related issues, comments, mentions, history, follow

**Wave 2 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** ISS-03, ISS-04, ISS-05, ISS-06, ISS-07, SEC-06 (history, comments, autocomplete), PRIV-12; UX design §4.1 (*Mentions*), §4.4 (*Related issues*, *History*), §5.1 (*Kinds of relation*); ADR-0007, ADR-0022, ADR-0027.

## Goal

Issues connect: goals hold their sub-issues and show progress, any two issues can be related in words both sides read, people talk in comments and mention each other, everybody sees what changed (only in the fields they may see), and people follow the issues they care about.

## Owns

`crates/bb-app/src/{relations,comments,history,follow}/**` (new), `crates/bb-store/src/{relations,comments,follow}.rs` (new) and migration `m0010` (`relation`, `relation_kind`, `comment`, `comment_version`, `mention`, `follower`), `crates/bb-api/src/api/{relations,comments,history,follow}*` (new), the history read model from the outbox, the sub-issues, related-issues and history slots of the issue page, *Organization settings › Kinds of relation*, the *Mentions* block on the start page, `userDocs/issues/*` (with WP-03).

## Tasks

1. **Sub-issues (ISS-03):** any issue can sit under a parent in any project the person may change; no circles; "Sub-issues · Add sub-issue" and "Part of: Goal …"; progress counts only the sub-issues the viewer may see (PRIV-13).
2. **Relations (ISS-04):** kinds with a name for each direction (starting: blocks / is blocked by, duplicate of / has duplicate, causes / is caused by, related to), editable with *Manage issue types, kinds of relation and layouts*; *Add relation* in three steps with Undo; across projects; a relation shows only when the viewer may see both issues; adding or removing it goes into both histories.
3. **Comments (ISS-05):** Markdown via WP-02; edited under WP-04's lock; every earlier version kept and shown to people who may see the comment; delete to the trash with Undo; *Comment* permission.
4. **Mentions (ISS-05, M3-Q7):** `@` autocomplete offers people and groups the writer may name (kind *Mentions*), roles for those who see roles only; stored as mention rows from WP-02's tokens; the *Mentions* block on the start page, unread first (M3-Q3). Issue-ID tokens become chips that resolve only for issues the reader may see; a hidden issue stays plain text.
5. **History (ISS-06, SEC-06):** a read model built from the domain events by an outbox consumer (idempotent, resumes from its place); *Everything · Comments · Changes*; each change filtered by the reader's mask (a change to a hidden field is left out, not shown as hidden); authors as `PersonRef`.
6. **Follow (ISS-07):** *Follow / Following*, *Followers 3* (names by kind *Followers*); people who report, are assigned or comment start following (one rule in `bb-core`); delivery comes in M4.

## Acceptance tests (write first)

- `iss_03_a_parent_cannot_become_its_own_sub_issue`, `iss_03_progress_counts_only_the_sub_issues_you_see` (both databases).
- `iss_04_a_relation_shows_from_both_sides_in_their_own_words`, `iss_04_a_relation_to_a_hidden_issue_is_not_shown`, `iss_04_adding_and_removing_is_in_both_histories`.
- `iss_05_an_edited_comment_keeps_its_earlier_versions`, `iss_05_a_guest_can_mention_a_role_but_not_a_person`, `iss_05_a_hidden_issue_id_in_a_comment_stays_text`.
- `iss_06_history_leaves_out_changes_to_fields_the_reader_cannot_see` (property test against the mask), `iss_06_the_history_reader_resumes_after_a_restart`.
- `iss_07_reporting_assigning_and_commenting_start_following`.
- Playwright: add a sub-issue and a relation, comment with a mention, see it in the other person's *Mentions*, read the history; four themes, axe.

## Checks

None new. ARCH-EVENTS (every command), ARCH-PERSON-REF (authors, followers, mentions), SEC-MATRIX rows for every route.

## Depends on

WP-02 (Markdown, tokens), WP-03 (issues), WP-04 (locks for comments), WP-06 (the slots it fills).
