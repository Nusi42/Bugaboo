# WP-05 — Policy engine: catalogue, roles, groups, assignments, field masks, `Authorized<T>`, `PersonRef`

**Wave 2 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** SEC-01, SEC-02, SEC-03, SEC-04, SEC-06, SEC-08, PRIV-12, PRIV-13 (engine side); ADR-0004, ADR-0022, ADR-0025 (WP-01); follow-ups M1-F18, M1-F32 (declaring), M1-F45 (the rule on the role), M1-F52, M1-F56.

## Goal

`bb-policy` makes every permission decision, and nothing else can: default deny, roles given to people and groups on the project tree, inheritance with *No access* (M2-Q1), field masks, `PersonRef` for every person shown, and `Authorized<T>` as the only way data leaves a use case. The M1 administrator flag is gone.

## Owns

`crates/bb-policy/**`; in `crates/bb-core/`: the new IDs (`GroupId`, `RoleId`, `AssignmentId`), `ShortName`, the field catalogue and the kinds of people information (new modules); in `crates/bb-app/`: the policy port, `Ctx` carrying the actor's evaluation, the use cases in `people/` and `roles/` (new) and the minimal *make a project* use case; in `crates/bb-store/`: migration `m0003`, the policy repository (new `policy.rs`); in `crates/bb-api/`: the actor extractor, the `x-permission` declaration on every route, the answer wrapper for `Authorized<T>`; every place that reads `administrator` (Rust and `web/app/src`); checks `xtask/src/checks/{arch_authz_only,arch_person_ref}.rs` and their fixtures.

## Tasks

1. **Catalogue and starting roles (SEC-01, SEC-08):** the permission enum with its message keys, topics, *where it counts* and kinds (the 14 permissions of architecture §7); the starting roles from one definition, made once as editable copies by a system use case at the first start (with events, never by the migration); *Reset to the starting permissions*.
2. **Storage (`m0003`, both databases):** `project` (ID, parent, short name, name, version), groups and members, roles and their permissions, assignments (person or group; role or *No access*; whole organization, project only, or project and sub-projects), field permissions (project, role, field, level), *Who sees people* rules (project, role, kind, level), the two-step switch of a role (M1-F45), which replaces the text setting `security.two-step-required-roles` (removed from the registry with its stored values). Personal-data marks as in 12-security.md, *Personal data in M2*. The M1 `administrator` column becomes an Administrator assignment on the whole organization and is dropped: the migration keeps the list, and the system use case that makes the starting roles gives those people the role in the same change (ADR-0025, decision 5).
3. **Evaluation (SEC-03):** load the actor's assignments for a project and its parents through one port call, evaluate in `bb-policy` without I/O (architecture §7, steps 1–4 as ADR-0025 makes them exact), cache per request. Access keys (WP-09) and AI-app tokens later pass their scopes in; the engine already intersects.
4. **Field masks (SEC-04, SEC-06):** for each field of the catalogue the highest level of the actor's roles (*Hidden < Can see < Can change*); `Authorized<T>` carries the mask, drops hidden fields when serialized and gives WP-02's `fields` helper the readable names, so a hidden field and an unknown one fail alike. Its constructor is private to `bb-policy`.
5. **People (PRIV-12, PRIV-13):** `PersonRef` resolved for the viewer, the kind and the project into a name, a role label (*a Team member*), *Someone*, or a random *Former user* label for a removed person (Crockford Base32, never counting, unique); the viewer always sees their own name.
6. **Check access (SEC-03):** a query use case that returns the decision for a person, a project and a permission or field, with the reasons (assignment, group, role, project, inherited from) as data; WP-12 turns them into sentences.
7. **Management use cases (no routes yet):** make, rename and delete groups and change members; make, change and delete roles; assign and remove roles and *No access*; set field permissions and *Who sees people*. Each asks the engine first, refuses a change that leaves nobody able to manage roles on the whole organization, and writes its domain event and a security-log entry. WP-11 and WP-12 put routes and screens on top.
8. **API:** the actor extractor builds the evaluation once per request; every route declares its permission in one place that also writes `x-permission` into the API description (M1-F32), including `public`, `signed-in` and `own-account` for the sign-in routes; `/auth/me` answers the person's whole-organization permissions instead of `administrator`; handlers return `Authorized<T>` through one answer wrapper. The browser app's menus and the administrator layout use the permissions.
9. **Tests from M1:** the generated test that runs every command use case against in-memory fakes and asserts its event (M1-F18); with those fakes, a test for the fallback when a security-log entry cannot be written (M1-F52); look again at `rename_table_for_tests` (M1-F56): remove it if the fakes cover the server failure tests, otherwise update the follow-up.

## Acceptance tests (write first)

- `sec_01_a_person_without_a_role_can_do_nothing_and_sees_no_project`.
- `sec_03_a_role_on_a_project_applies_to_its_sub_projects_unless_given_for_this_project_only`, `sec_03_no_access_on_a_sub_project_wins_over_a_role_from_above`, and the same-project rule of M2-Q1.
- Property tests: a new role never takes a permission away; *No access* never adds one; any scope list gives at most the owner's permissions.
- `sec_04_the_highest_field_level_of_the_roles_wins`, `sec_06_a_hidden_field_is_not_in_the_answer_and_asking_for_it_fails_like_an_unknown_field`.
- `sec_08_starting_roles_are_made_once_and_can_be_changed_and_reset`, `sec_08_nobody_can_remove_the_last_way_to_manage_roles`.
- `priv_12_people_are_shown_by_name_role_or_someone_as_the_rules_say` (every kind × level, inherited down the tree), `priv_13_former_user_labels_are_random_and_unique`.
- Migration on both databases: an M1 database with an administrator signs in after the upgrade and still opens *Organization settings*.
- Every management use case writes one event and one security-log entry; the log verifies.

## Checks

- **ARCH-AUTHZ-ONLY** switched on: a compile-fail test builds `Authorized<T>` outside `bb-policy`, and an ast-grep rule finds other ways to construct it; bad and good fixtures.
- **ARCH-PERSON-REF** switched on: an ast-grep rule finds names and e-mail addresses in API views outside `PersonRef` (the person's own account is listed with a reason), and a serialization test reads the API description; bad and good fixtures.
- ARCH-EVENTS gains its generated test (M1-F18).

## Answers

The maintainer accepted M2-Q1, M2-Q3, M2-Q5 and M2-Q7 as proposed. Build from [ADR-0025](../../adr/0025-roles-groups-and-assignments.md) and [architecture §7](../../architecture/07-cross-cutting.md#authorization-model):

- **Evaluation (M2-Q1):** rank by depth, then person before group; the highest *No access* removes every role row of equal or lower rank; *Decide who has which role* from a removed row is kept (no lock-out). Add `sec_03_no_access_cannot_lock_out_the_people_who_decide_roles_above` and `sec_03_a_role_taken_away_counts_from_the_next_request`.
- **No escalation:** nobody adds a permission to a role, or gives a role, unless they hold all of it at that place: `sec_08_nobody_adds_a_permission_they_do_not_hold_to_a_role`, `sec_02_nobody_gives_a_role_with_more_than_they_hold_there`.
- **Starting roles (M2-Q5):** the permissions, field levels, *Who sees people* levels and order of the tables in §7. The upgrade test is `sec_08_m1_administrators_become_administrators_and_nobody_else_does`. Setup stays open while nobody can manage roles for the whole organization.
- **People (M2-Q3):** the kinds of people information including *Project members*, and the rule that people who manage people or roles see names in the management screens; `PersonRef` as the tagged union in §7 (a role label and *Someone* carry no ID).
- **Fields (M2-Q7):** the 16 built-in fields of §7, with their keys and message keys `issue-field.<key>`.
- **Security log:** `sec_15_every_change_of_permissions_is_in_the_security_log` runs every management use case.

## Depends on

WP-01 (ADR-0025, field catalogue, kinds of people information), WP-02 (`fields` helper), WP-03 (the browser menus the permissions now drive).
