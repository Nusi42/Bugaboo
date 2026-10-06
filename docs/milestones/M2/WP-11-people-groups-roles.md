# WP-11 — *Organization settings*: People, Groups, Roles & permissions

**Wave 4 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** SEC-02, SEC-03 (*Who has which role*), SEC-08, PRIV-12 (names in these screens, M2-Q3); UX design §5.1, §5.2, §6; ADR-0025.

## Goal

Administrators manage people, groups and roles in plain words, and project leads decide who has which role in their projects, seeing which rows come from a parent project and changing them there. This package puts routes and screens on WP-05's use cases.

## Owns

`crates/bb-api/src/api/{people,groups,roles,assignments}*` (new), the query use cases these screens need in `crates/bb-app/src/{people,roles}/` (lists, details, *Used by*), `web/app/src/features/{people,roles}/**` (new), the tab *Who has which role* under `/p/<short name>/settings/roles`, the move of the M1 invitations and sign-up pages into *People*, `userDocs/people/*` (new).

## Tasks

1. **People:** a searchable, virtualized, paged list (name, groups, where they have which role); *Invite* (the M1 invitations move here, with the sign-up rule); a person's page with their groups and roles. E-mail addresses only for people allowed by the catalogue; names as M2-Q3 decided, through `PersonRef`.
2. **Groups:** a list → editor; make, rename, delete (with *Used by*: "Used in 4 projects", counting only what the viewer may see); add and remove members with a searchable picker (recent choices first, UX §6).
3. **Roles & permissions:** a list of roles; the role page with permissions grouped by topic, each a checkbox with its plain sentence; the two-step switch (WP-05, enforced by WP-09); *Reset to the starting permissions* for starting roles; make, copy, rename and delete roles (*Used by* before deleting; a role in use cannot be deleted without choosing what replaces it).
4. **Who has which role:** whole-organization assignments in *Roles & permissions*; per project in *Project settings*, with rows from a parent project marked *from ACME*, each changeable for this project, and *No access* as M2-Q1 decided; scope choice (*Only this project*, *This project and its sub-projects*). Changes that take a person's own way to manage roles away ask first; the change that would leave nobody able to manage roles on the whole organization is refused with a sentence that says why.
5. **Plain words and undo:** reversible changes happen at once with *Undo* (10 seconds); deleting a group or role says exactly what happens (principle 7); a Project lead sees only their projects.
6. **Exit demo, first step:** an administrator makes a role from a copy and changes its permissions.

## Acceptance tests (write first)

- API tests on both databases for every route, including refusals: a Team member cannot open *Organization settings*; a Project lead manages roles only in their projects; deleting a role in use asks for a replacement.
- `sec_02_people_and_groups_get_roles_on_the_whole_organization_a_project_or_a_project_and_its_sub_projects`.
- `sec_08_a_changed_starting_role_can_be_reset`.
- `priv_13_used_by_counts_only_what_the_viewer_may_see`.
- Playwright: invite a person, make a group and add members, copy a starting role and change it, give the group the role on a project, see the inherited row on a sub-project and set *No access* there, get the refusal for the last administrator; on the phone too; four themes, axe; screenshots.

## Checks

None new; TEST-A11Y covers the new addresses, ARCH-PERSON-REF the people in answers; every route declares its permission (ARCH-PERMS) and adds its SEC-MATRIX example.

## Answers

- ***No access* (M2-Q1):** set per person or group on a project, always for the project and its sub-projects; it wins over roles from above and over a group's role on the same project; a role given lower down still counts. People who decide roles higher up keep seeing the project's name in *Who has which role*, so they can take a *No access* away again ([ADR-0025](../../adr/0025-roles-groups-and-assignments.md), decisions 3 and 6).
- **Names (M2-Q3):** people with *Manage people*, *Manage groups*, *Manage roles* or *Decide who has which role* at that place see names in these screens; everybody else sees people by the kind *Project members*. E-mail addresses only with *Manage people*.
- **Starting roles (M2-Q5):** the tables in [architecture §7](../../architecture/07-cross-cutting.md#authorization-model); nobody gives a role, or adds a permission to one, that they do not hold themselves at that place (the refusal says so in a sentence).
- **Choices for project leads (M2-F86, maintainer decision):** a project lead may pick from every person and group of the organization when giving roles in their project; the role choices show their names. No code change.

## Depends on

WP-05 (use cases, policy), WP-07 (settings form for the sign-up rule), WP-08 (projects for assignments), WP-10 (matrix examples table, checks).
