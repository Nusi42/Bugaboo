# ADR-0025: Roles, groups and assignments on the project tree

Status: **Accepted** · 2026-10-05 · refines [ADR-0004](0004-authorization.md) and [ADR-0022](0022-who-sees-people.md)

## Context

ADR-0004 says that one policy engine (`bb-policy`) makes every decision and that data leaves the domain only as `Authorized<T>`. ADR-0022 says that people are shown through `PersonRef` by the rules of *Who sees people*. Neither says what exactly a role holds, where a role counts, what wins when two rules disagree, or how the M1 administrator flag turns into roles. M2 builds all of it (SEC-01, SEC-02, SEC-03, SEC-04, SEC-06, SEC-08, AUTH-07, PRIV-12, PRIV-13), and the open questions M2-Q1 … M2-Q7 were answered by accepting every proposal. This record writes the model down so that WP-05 can build it without asking; the details a builder needs (the tables, the shapes of answers) are in [architecture §7, *Authorization model*](../architecture/07-cross-cutting.md#authorization-model).

## Decision

### 1. One permission catalogue

- **One Rust enum, `Permission`, in `bb-policy`**, is the only list of permissions (coding standards §1). Every permission has a key (`see-projects`), a **topic** (*Projects, People, Settings*; later milestones add *Issues, Documents, Time*), a plain sentence as a message key (`permission.see-projects`: "See the project and what is in it"), **where it counts** and a **kind** for access keys. The API description, the role page, SEC-MATRIX and the docs are made from the enum. Enforced by: ARCH-PERMS (every route declares one catalogue key or `public`, `signed-in`, `own-account`, `by-record`; every permission is used and has its message), SEC-MATRIX.
- **Where it counts:** an *organization* permission (make top-level projects, people, invitations, groups, roles, organization settings, the status page) counts only from a role given for the whole organization; a *project* permission counts in projects. A role given on a project that holds an organization permission gives nothing for it, and the role page says so.
- **Kinds** (for access keys, decision 8): *See* (changes nothing), *Change* (work in projects), *Manage* (people, roles, settings).
- The M2 catalogue has 14 permissions (the table is in architecture §7). A later milestone adds its permissions to the enum with their sentences, kinds and starting values in the same change (issues M3, documents M11, time M6); a permission no route uses fails ARCH-PERMS.

### 2. Assignments: who has which role, and where it counts

- An **assignment** is one row of *Who has which role*: a **person or a group**, a **role or *No access***, and **where it counts**: *Whole organization*, *Only this project*, or *This project and its sub-projects* (SEC-02).
- *Whole organization* rows exist only in *Organization settings*; project rows only on a project. *No access* is set only on a project and always covers *This project and its sub-projects* (M2-Q1).
- A person or group may hold several roles on one project, but never a role and *No access* on the same project: such a change is refused with a sentence that says which row is in the way.
- Groups are flat in M2 (nested groups are L). A person's groups are read once per request with their assignments.

### 3. Evaluation order and *No access*

For an actor and a place (the organization, or a project with its parents), `bb-policy` decides without I/O, from one port call that loads the actor's assignments for the place and its parents:

1. Nobody signed in: only `public` routes. Signed in without an applicable assignment: nothing (default deny, SEC-01).
2. **Applicable** are the actor's own and their groups' assignments for the whole organization, those on a parent of the project with *This project and its sub-projects*, and those on the project itself.
3. Each applicable row has a **rank**: first the depth of its project (whole organization lowest), then the person's own row above a row of one of their groups.
4. ***No access* (M2-Q1):** the highest-ranked *No access* row removes every role row of equal or lower rank. So it wins over roles given higher up, over a role of one of the person's groups on the same project, and over another group's role on the same project. A role given lower down (on a sub-project below the *No access*), or given to the person directly on the same project where a group has *No access*, still counts: the nearer, more personal row decides.
5. The permissions are the union of the remaining roles, filtered by *where it counts*; then an access key narrows them (decision 8).
6. The field mask and the *Who sees people* levels are worked out from the same remaining roles (decisions 10 and 11).

Evaluations are cached for one request and never across requests, so a role taken away counts from the next request. Enforced by: review (the cache is a field of the request context, which a reviewer sees in the diff), backed by the tests `sec_03_no_access_on_a_sub_project_wins_over_a_role_from_above`, `sec_03_a_role_taken_away_counts_from_the_next_request` and the property tests of WP-05 (a new role never takes a permission away; *No access* never adds one), and SEC-MATRIX.

### 4. Starting roles are editable copies

- **One Rust definition** in `bb-policy` holds the six starting roles of SEC-08 and M2-Q5 (*Administrator, Project lead, Team member, Contributor, Viewer, Guest*): their permissions, field levels, *Who sees people* levels and order (architecture §7). A system use case makes editable copies from it once, at the first start, with events and security-log entries (never in a migration). After that the definition is read only by **Reset to the starting permissions**, which a copy keeps through a `starting` mark that renaming does not remove.
- A starting role can be renamed, changed and deleted like any other role; the last-administrator rule (decision 6) still holds. Enforced by: the tests `sec_08_starting_roles_are_made_once_and_can_be_changed_and_reset` and `sec_08_a_changed_starting_role_can_be_reset`.
- The **order of roles** (the starting order, then new roles where an administrator puts them) decides the label of a person shown as *Role only* when they hold several roles: the first in the order.

### 5. The administrator flag becomes the Administrator role

- Migration `m0003` keeps the list of M1 administrators. In the same change that makes the starting roles at the first start, each of them gets the **Administrator role for the whole organization**, with its event and security-log entry; afterwards the flag no longer exists in the database, the API (`/auth/me` answers the person's organization permissions instead) or the browser app. Enforced by: the test `sec_08_m1_administrators_become_administrators_and_nobody_else_does` (both databases).
- **Setup (AUTH-09) stays open while nobody can manage roles for the whole organization** (decision 6 makes that the state of a fresh installation only). Making the first administrator makes the starting roles if they do not exist yet and gives the person the Administrator role for the whole organization, in one change.

### 6. Nobody locks the organization out, and nobody gives what they do not have

- **The last administrator:** a change that would leave no person who holds both *Manage roles* and *Decide who has which role* for the whole organization (through their own row or a group) is refused, with a sentence that says why. This covers removing a row, taking a person out of a group, deleting a group or role, and taking a permission out of a role. Enforced by: the test `sec_08_nobody_can_remove_the_last_way_to_manage_roles`.
- ***No access* never locks out the people above:** *Decide who has which role* on a project is kept for a person who holds it through a row that the project's *No access* removed, **only when that row is strictly above the *No access*** (a parent project, or the whole organization). A row at the depth of a *No access* that removes it, such as a role of the person's group next to their own *No access* on the same project, is removed as a whole: otherwise a person could lift their own *No access*. They see the project's name and short name in *Who has which role* and can take the *No access* away again; they see nothing in the project. Enforced by: the tests `sec_03_no_access_cannot_lock_out_the_people_who_decide_roles_above` and `sec_03_decide_roles_is_kept_only_from_rows_strictly_above_the_no_access` (engine), and `sec_03_decide_roles_is_not_kept_from_a_row_at_the_same_depth_as_the_no_access` (both databases).
- **No escalation:** a person can add a permission to a role, or give a role, only when they hold every permission of it at that place themselves (organization permissions are left out of this comparison for a role given on a project, because they count nowhere there). A role given *with its sub-projects* reaches the sub-projects made later as well as those that exist, so the giver holds it through rows that reach sub-projects themselves (their rows for the project alone do not count) **and in every sub-project that exists** (a lead for one project only cannot make themselves lead of everything below it, now or when a sub-project is made). Putting a person into a group gives them every role of the group, so the same holds for each row of the group. The check runs inside the change, after the policy lock (making a project takes the lock too), so two changes cannot both pass on what was stored before; the same holds for adding to a role and for resetting one, which read the role and what the person holds there. A level is held to the same rule: taking a project's setting away is setting the level that then applies there (the nearest parent's, else the role's own). Enforced by: the tests `sec_08_nobody_adds_a_permission_they_do_not_hold_to_a_role`, `sec_02_nobody_gives_a_role_with_more_than_they_hold_there`, `sec_02_nobody_gives_a_role_with_sub_projects_they_do_not_hold_there`, `sec_02_a_role_with_sub_projects_needs_a_row_that_reaches_sub_projects`, `sec_02_a_group_with_a_role_for_sub_projects_is_not_joined_by_a_lead_of_one_project` and `sec_04_nobody_raises_a_level_by_taking_an_override_away`.
- ***Manage groups* does not lift a *No access*:** taking a person out of a group, or deleting a group, that has *No access* rows takes those rows away for them, so it needs *Decide who has which role* at each project where the group has one, like taking the row away itself. Giving, adding to a group, taking out of a group and deleting a group are the four ways a person's rights change through a group. Enforced by: the test `sec_02_manage_groups_does_not_lift_a_no_access_of_the_group`.

### 7. Two-step sign-in is a switch on the role

- A role has a switch **Two-step sign-in for everyone with this role** (AUTH-03, M1-F45), stored with the role; the text setting `security.two-step-required-roles` is removed with its stored values (WP-05). `security.two-step-required` stays: it asks it of everybody.
- A person needs a second step when the organization switch is on or any role they hold anywhere (own row or group, any place) has the switch on. It takes effect at the next request: until the person has a code app or a passkey, their session can only set one up. Enforced by: the test `auth_03_a_role_that_requires_two_step_sign_in_makes_its_people_set_it_up_first` (WP-09).
- **Access keys follow it** (M2-F46): while a person is asked for a second step, the keys they made are refused too, with 403 `auth.second-step-required` and an entry in the security log, and work again as soon as they have set one up. Without this, an old key of a person who is asked for a second step would still work and could even switch the requirement off for its owner. Enforced by: the tests `auth_03_a_key_made_before_the_switch_is_refused_until_its_owner_sets_up_a_second_step` and `auth_03_a_key_cannot_switch_off_the_requirement_of_its_owner` (WP-09).

### 8. Access keys can only narrow

- *What it may do* (M2-Q6) is one of three choices: **See only** (permissions of kind *See*, every field at most *Can see*), **See and change** (*See* and *Change*), **Everything I can do** (all kinds); under *More options*, single permissions instead. Optionally a list of **projects**: the key then counts only in them and their sub-projects (by project ID, so a move does not change it) and gives nothing for the whole organization.
- At **every use** the key's permissions are the owner's evaluation at that place intersected with the key's choice and projects; a key never widens anything, and follows its owner's roles at once (AUTH-07). No key reaches the owner's own sign-in data (password, passkeys, two-step, sessions, access keys), and a key is refused while its owner is asked for a second step (decision 7).
- **What no key holds** (decision): whatever its choice, even *Everything I can do* and single permissions, a key never keeps *Invite people*, *Manage people*, *Manage groups*, *Manage roles*, *Decide who has which role*, *Decide what roles see* and *Change organization settings* (lead decision, so that a key cannot switch off the two-step requirement, open signing up or change how long sessions last). Later decisions added five: *Edit anyway* (ADR-0019), *Manage issue types, kinds of relation and layouts*, the permission to create, change and retire fields, *Manage labels and tags* (M3-Q6), and *Manage workflows* (ADR-0031), so the list in the code has twelve entries. They stay in the app, where the person is signed in, so a stolen key cannot invite an administrator, take roles away or change the settings of the organization. The list is defined once beside the kinds, in `bb-policy` (`NOT_FOR_KEYS`); narrowing removes them, the key form does not offer them and the server refuses a key made with one. Enforced by: the tests `auth_07_a_key_never_invites_people_or_changes_roles_whatever_its_choice` (it also tries the settings of the organization), `auth_07_single_permissions_of_a_key_never_include_those_a_key_never_holds` and the property test `auth_07_a_key_can_do_only_what_its_owner_can_and_its_scopes_allow` (WP-09).
- **Keys end with *Sign out everywhere*** (decision): the person's keys are deleted in the same change, each one with an entry in the security log. A password change ends the other sessions and leaves the keys. Enforced by: the test `auth_07_sign_out_everywhere_ends_the_keys_and_a_password_change_leaves_them` (WP-09).
- **Every key runs out**: 90 days unless the person picks another date, never later than the organization's longest time (an organization setting, default and most 365 days). The settings are declared in the registry by WP-09. Enforced by: the tests `auth_07_a_key_can_do_only_what_its_owner_can_and_its_scopes_allow` (property test), `auth_07_a_key_follows_its_owner_losing_a_role_at_once` and `auth_07_a_key_cannot_change_the_owners_sign_in_data`, and SEC-MATRIX (an access key with *See only*, and one for another project).

### 9. Every permission change is in the security log

Every change of roles (permissions, two-step switch, name, reset), groups (made, renamed, deleted, members), assignments (including *No access*), field levels, *Who sees people* levels, project moves and short names, and access keys (made, deleted, refused) writes its domain event **and** a security-log entry in the same change (SEC-15). Enforced by: ARCH-EVENTS, and the generated test `sec_15_every_change_of_permissions_is_in_the_security_log` that runs every management use case (WP-05).

### 10. Field masks

- The **field catalogue** (built-in fields of M2-Q7) is declared once in `bb-core`; own fields join in M3 (CF-06). Each field has a level per role: *Hidden < Can see < Can change*. The organization level is the role's own level; a project can set another level for a role, which its sub-projects inherit until one of them sets its own.
- The mask is the **highest level of the remaining roles** (decision 3), capped at *Can see* for a *See only* key. `Authorized<T>` carries it and leaves hidden fields out when it is serialized; a hidden field and an unknown one are refused alike. Enforced by: ARCH-AUTHZ-ONLY, ARCH-API-OUT, SEC-MATRIX, and the test `sec_06_a_hidden_field_is_not_in_the_answer_and_asking_for_it_fails_like_an_unknown_field`.

### 11. People: `PersonRef` and the kinds of people information

- *Who sees people* (ADR-0022) is set per role and per **kind of people information**; M2 adds the kind **Project members** (M2-Q3): member lists of a project. Levels are inherited like field levels; the viewer's level is the most open of their remaining roles (*Names > Role only > Hidden*).
- **People who manage see names (M2-Q3):** in *Organization settings*, *Who has which role* and *Check access*, a viewer who holds *Manage people*, *Manage groups*, *Manage roles* or *Decide who has which role* at that place always sees names; everybody else sees the people there by the kind *Project members*. E-mail addresses are part of no `PersonRef`: only *Manage people* and the person themselves see them.
- `PersonRef` resolves for the viewer, the kind and the place into a name, a role label, *Someone*, or a random *Former user* label (PRIV-13); the viewer always sees their own name. A role label and *Someone* carry no ID of the person. Enforced by: ARCH-PERSON-REF, SEC-MATRIX (people rows), and the tests `priv_12_people_are_shown_by_name_role_or_someone_as_the_rules_say` and `priv_13_former_user_labels_are_random_and_unique`.

### 12. Check access answers with reasons

*Check access* is a query use case: for a person and a place it lists every permission that counts there with *yes* or *no*, every field with its level and every kind of people information with its level, each with its **reasons** as data (the row, the person or group, the role, the project it is on, whether it is inherited, a *No access* that decided, or that no role gives it). The browser turns the reasons into sentences from message keys. A reason never names a project the checker cannot see: it says the row is on *a project you can't see*. Enforced by: the property test `sec_03_check_access_explains_every_result_with_the_chain_of_reasons` (the decision equals the engine's and the reasons lead to it) and `sec_03_check_access_names_no_project_the_checker_cannot_see`.

### 13. Projects and short names

- A **short name** (M2-Q4) is 2 to 10 capital letters or digits and starts with a letter (`WEB`, `APP2`); it is unique in the organization, old short names included, and can be changed. An old short name stays with its project, so its addresses keep working (`project_alias`, like `issue_alias` in ADR-0021); only that project can take it back. One definition, `ShortName` in `bb-core`. Enforced by: the tests `prj_01_a_short_name_breaks_no_rule_of_m2_q4` and `prj_01_an_old_short_name_redirects`.
- **Hidden projects stay hidden:** a project the person cannot see answers `resource.not-found` like a missing one, by address and by short name; a visible sub-project of a hidden project is shown at the top and never names its parent. A short name that is taken is told only to people who may create projects at that place, with the same sentence whether the project is hidden or not. Enforced by: SEC-MATRIX, the tests `prj_01_a_hidden_project_answers_not_found_like_a_missing_one`, `prj_01_a_visible_sub_project_of_a_hidden_project_does_not_name_it` and `prj_01_a_taken_short_name_is_told_only_to_people_who_may_create_projects`.

### 14. Who may change a setting

The registry's `changed_by` (ADR-0018) maps to the catalogue: *Administrators* → *Change organization settings*; *Project leads* → *Change project settings* on that project; *Each person* → only their own value. Enforced by: ARCH-SETTINGS, and the test `cfg_06_only_administrators_change_organization_values_and_each_person_only_their_own` (WP-07).

## Consequences

- WP-05 builds the engine from this record and architecture §7; WP-08, WP-09, WP-11 and WP-12 put screens and routes on it; SEC-MATRIX (WP-10) runs every route for every starting role, a role on a parent and on a sibling project, *No access* and two access keys.
- The deny rule has one exception (decision 6), so administrators cannot be shut out of a sub-project for good; the people above see that the project exists, but not what is in it.
- Group membership and role assignments are personal data of a new category, *organization* (where a person works); access keys and the person's own settings are personal data too. The marks are listed in [12-security.md](../architecture/12-security.md#personal-data-in-m2) and come from the code into the generated personal-data page when WP-05, WP-07, WP-08 and WP-09 add the entities. Enforced by: DATA-MARKS.
- *Check access* tells a checker that a role comes from *a project you can't see*, so a hidden parent's existence (never its name) shows there.
- Removing the M1 flag touches every place that read `administrator`, in Rust and in the browser app (WP-05).

## Alternatives considered

- ***No access* that wins in its whole subtree, even over roles given lower down:** simpler, but an organization could then not hide a big project from a group except for one sub-project. Rejected; the nearer row decides, as for field levels.
- ***No access* only for the people it names, never over a group's role at the same project:** a person could not be taken out of one project without leaving their group. Rejected (M2-Q1).
- **Hard-coded roles:** SEC-08 asks for editable copies. Rejected.
- **Reading the starting definition at every evaluation (roles as overrides of the definition):** a change of the definition in an update would silently change every organization's roles. Rejected; copies are made once.
- **Access keys with their own role:** keys could then hold more than their owner. Rejected (AUTH-07: keys only narrow).
- **The deny as a role (*No access* is a role with no permissions):** a role adds, it cannot take away. Rejected.

## Enforced by

ARCH-PERMS, ARCH-AUTHZ-ONLY, ARCH-API-OUT, ARCH-PERSON-REF, ARCH-NO-COUNTERS, ARCH-EVENTS, ARCH-SETTINGS, DATA-MARKS and SEC-MATRIX (see `tools/checks/catalog.toml`), backed by the named tests of WP-05, WP-08, WP-09, WP-11 and WP-12; the per-request cache by review.
