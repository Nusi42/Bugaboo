# ADR-0034: Saved searches: owned by one person, shared without giving anyone more

Status: **Accepted** · 2026-10-25 · refines QRY-06, CFG-02, SEC-06, PRIV-13 and [ADR-0005](0005-search-language.md); decisions M5-Q1, M5-Q2, M5-Q8, M5-Q12

## Context

M5 lets people save a search with how its results are shown, and share it with groups or projects (QRY-06, CFG-02). A shared search is a piece of text written by one person and read by others. It must never let a receiver see an issue, a field, a value or a person that their own rights hide, and its text must not tell them that something hidden exists.

## Decision

1. **What a saved search is:** a name, the canonical search text (ADR-0005, amendment M5), how it is shown (the kind of view, columns with their order and widths, *Spacing*, grouping), the person who made it, and the groups and projects it is shared with. It has a stable address, `/s/<id>`. M5 shows the kind *list*; board, timeline and calendar read the same record when their milestones come (M5-Q12).
2. **Private when it is made.** Sharing it needs the new permission *Share searches* (counts per project; Administrator, Project lead, Team member, Contributor and Viewer start with it, Guest does not):
   - with a project: *Share searches* in that project;
   - with a group: *Share searches* for the whole organization and being in that group, or *Manage groups*.
   The *Share* window lists only the groups and projects the person may share with (UX design §4.3).
3. **It always runs with the rights of the person who opens it** (QRY-05). Two people may see different issues in the same search, and being shared with never adds rights.
4. **A receiver sees a shared search only when its text checks cleanly against their own rights:** every field it names is one they may read, and every person, group and project it names is one they may see. Otherwise it is not listed for them, and its address answers exactly like an address that does not exist. Its name and text never reach them.
5. **Only the person who made it changes, shares or deletes it.** A person with *Change organization settings* may delete any saved search. Everyone who sees it may *Save a copy*, which is private and their own.
6. **A project's default list** (M5-Q2): a person with *Change project settings* may choose a saved search shared with the project as the project's default list. Without one, the project shows the built-in default: the columns of UX design §4.2 (status, ID, type, title, labels, priority, assigned, changed), grouped by status, *Comfortable*. A project's list and the *Search* page are the same screen; the project's list is the search with `within KEY` filled in (M5-Q8).
7. **Changes are events** (ADR-0007): made, changed, shared, no longer shared, deleted. Deleting goes to the trash with *Undo*, like issues.
8. **Personal data:** the name and the text are content written by a person (marked as such). When the person who made a search is removed, their private searches go with them; a shared search stays, made by *Former user …*, and a person with *Change organization settings* may delete it.

## Consequences

- The checker of the search language runs twice for a shared search: once for the person who saves it, and once for each receiver when it is listed or opened. It is pure and fast, so this costs little.
- A receiver whose rights shrink loses the search from their list without a message.
- Saved searches are the base of later features: standing rules for agents (ADR-0029), dashboards and e-mailed searches (QRY-07, V1).

## Alternatives considered

- **Shared searches run with the rights of the person who made them:** a receiver would see what only the maker may see. Rejected (SEC-06).
- **Show a shared search with a hidden field as "This search uses something you cannot see":** it tells the receiver that the field exists, which is what an unknown field must never tell (QRY-05). Rejected.
- **Anyone may edit a shared search:** a search changes under the people who rely on it. Rejected; *Save a copy* covers the need.

## Amendment M5 WP-06: how it is stored and served

The builder of M5 WP-06 decided the following where the decision above left room:

1. **Tables** (migration 23): `saved_search`, its columns `saved_search_column` (the search word of each field, its place and its width), its shares `saved_search_share` (a group or a project, told apart by `kind`; no reference for the target, so deleting a group takes its rows with it in the same change) and `project_default_list` (one row for each project). The maker has no reference to the person (a removed person leaves a tombstone, PRIV-04).
2. **Spacing is a choice of the setting *Spacing*** (`appearance.spacing`): the registry holds its two words once and a saved search stores one of them. The widths are whole numbers in the units of the list; a list holds at most 40 columns and a saved search is shared with at most 100 places.
3. **A saved search is the person's own data:** only a session reaches the routes, never an access key (ADR-0025, decision 8).
4. **Who is reached:** a share with a group reaches the people in it, a share with a project the people who see it. The list *Shared with me* holds the searches of other people that a share reaches, oldest first, and the person's own are in the list *Mine*. A person who is not the maker, does not see the share's place, or looks at one in the trash, gets `resource.not-found`, with the same reads as for an address that does not exist.
5. **The maker is shown** as the viewer sees *Authors*, by the rights the viewer has in the first project that reaches them (else in the organization). Whom a search is shared with is told to its maker only.
6. **The project's default list** is the row of `project_default_list`. It is read only while the search is not in the trash, is still shared with the project and the reader may open it; stopping the share takes the row away in the same change, and choosing none takes it away too. The change is an event about the project (`default_list`).
7. **A person who is removed:** `delete_private_searches_of` (the port) deletes the searches shared with nobody in the change that removes the person; the shared ones stay, and the maker's name is *Former user …* because the name of the person is.

Enforced by (additional): the tests `qry_06_*` of `bb-core`, `bb-app`, `bb-store` and `bb-server`.

## Enforced by

SEC-MATRIX (the `search` entry point, with saved searches), ARCH-PERMS (*Share searches*), ARCH-EVENTS, DATA-MARKS, PERF-N1 (the list of saved searches), and the tests of M5 WP-06 named in the security chapter.
