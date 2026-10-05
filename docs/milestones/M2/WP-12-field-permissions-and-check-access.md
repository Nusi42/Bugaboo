# WP-12 — Field permissions, *Who sees people* and *Check access*

**Wave 4 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** SEC-04, SEC-06, SEC-03 (*Check access*), PRIV-12, PRIV-13; UX design §5.2 (*Field permissions*, *Who sees people*, *Check access*); ADR-0004, ADR-0022, ADR-0025.

## Goal

Administrators and project leads set, per role and per project, which fields each role may not see, see or change, and how each role sees people; anybody allowed to check access chooses a person and a project and reads what that person can do, and why, in sentences.

## Owns

`crates/bb-api/src/api/{field_permissions,people_visibility,check_access}*` (new), the query use cases behind them in `crates/bb-app/src/access/` (new), `web/app/src/features/access/**` (new; its pages appear under *Roles & permissions* through added menu lines), the tabs under `/p/<short name>/settings/{fields,people}` and `/settings/check-access`, the sentence messages of *Check access*, `userDocs/permissions/*` (new).

## Tasks

1. **Field-permission matrix (SEC-04):** rows are the built-in fields of the catalogue (M2-Q7), columns the roles, each cell a three-choice list *Hidden / Can see / Can change*; on a project, cells inherited from a parent say so and can be changed for this project or reset; on a phone the matrix becomes a list per role. Rules per issue type come with issue types (M3).
2. **Who sees people (PRIV-12):** the kinds of people information × roles, each *Names / Role only / Hidden*, on the organization and per project, inherited the same way; the screen says in one line what a Guest will see (*"Assigned to a Team member"*).
3. **Check access (SEC-03):** choose a person and a project; the answer lists, by topic, what the person can do, every field with its level, and how they see people, each with a sentence built from WP-05's reasons ("Yes. Ana is in the group Backend, which has the role Team member in ACME. Website is inside ACME, so the role applies here too."). Names in the sentences as M2-Q3 decided; nothing about projects the checker may not see.
4. **Everywhere a value could leak (SEC-06, M2 part):** answers through `Authorized<T>`, `fields=` refusing hidden fields like unknown ones, *Check access* itself, and the people its own screens show to a viewer with *Hidden*; write which later entry points (search, e-mail, exports, assistant) still have to add their SEC-MATRIX rows, in the README's coverage list if anything changed.
5. **No counts that leak (PRIV-13):** none of these screens shows a number that includes hidden people, fields or projects.
6. **Exit demo, steps 2 and 3:** hide *Cost center* from Contributors and names from Guests in a project, then check Ana's access and read the explanation.

## Acceptance tests (write first)

- `sec_04_a_field_level_set_on_a_project_applies_to_its_sub_projects_and_can_be_changed_below` (both databases).
- `sec_06_a_hidden_field_is_missing_from_every_m2_answer_and_cannot_be_asked_for`.
- `priv_12_a_guest_sees_role_only_or_someone_as_set_and_no_name_reaches_their_browser` (the answer bodies are searched for the names).
- `sec_03_check_access_explains_every_result_with_the_chain_of_reasons` (property test: the decision equals the engine's, the reasons lead to it).
- Playwright: set *Cost center* to *Hidden* for Contributors on a project and see it inherited on a sub-project; set names to *Hidden* for Guests and sign in as a Guest to see *Someone*; check a person's access and read the sentence; on the phone too; four themes, axe; screenshots.

## Checks

None new; SEC-MATRIX field and people rows (WP-10) cover the engine; this package adds the examples of its routes. TEST-A11Y covers the new addresses.

## Answers

- **Fields (M2-Q7):** the rows of the matrix are the 16 built-in fields of [architecture §7](../../architecture/07-cross-cutting.md#authorization-model), *The field catalogue*, with the starting levels there (a Contributor starts with *Can see* on *Cost center*, so the exit demo can hide it).
- **Who sees people (M2-Q3):** the kinds include *Project members* (starting levels in §7); managers see names in *Check access* as WP-11's screens do.
- ***Check access*:** the answer and its reasons have the shape written in §7; a reason on a project the checker cannot see says *a project you can't see* ([ADR-0025](../../adr/0025-roles-groups-and-assignments.md), decision 12). Add `sec_03_check_access_names_no_project_the_checker_cannot_see`.

## Depends on

WP-05 (masks, `PersonRef`, explanation), WP-08 (projects), WP-10 (matrix, checks), WP-07 (settings form patterns). Runs beside WP-11 in folders of its own; both add lines to the *Organization settings* and *Project settings* menus.
