# ADR-0027: Layouts, rules per issue type and the starting workflow

Status: **Accepted** · 2026-10-09

## Context

M3 brings issue types (ISS-02), layouts for the issue page and the new-issue form (CFG-01), field levels per issue type (SEC-04, UX §5.2 *More options*), kinds of relation (ISS-04) and own fields (CF-03). All of these are set for the whole organization or a project, and some also for one issue type. The settings registry (ADR-0018) has the layers default → organization → nearest project → person, but no issue type, and a layout is a structure, not a value. Workflows come in M4, yet every issue needs a status from the start.

## Decision

1. **Own tables, settings-like lookup.** Layouts, field levels per issue type and kinds of relation live in their own tables, not in the settings registry. Each row names its place (the organization or a project) and optionally an issue type.
2. **One lookup order, defined once:** for a project and an issue type, the nearest place with a row wins (the project, then its parents, then the organization); at one place, a row for the issue type wins over a row for all types. The lookup is one function in `bb-core`, used by layouts and by field levels; a sub-project uses its parent's layout until it gets its own.
3. **Field levels per issue type** join the evaluation of architecture §7 (step 7) through that lookup; a level for a type is set under *More options* on the field-permission page.
4. **Layouts never widen permissions.** A layout decides where a field shows and for which roles; a field shows only if the layout lists it for one of the person's roles and the field level lets them see it (to fill it in on the new-issue form: change it).
5. **The starting workflow** (decision M3-Q1): one workflow, defined once in `bb-core`, with *Open* (status group *Not started*), *In progress* (*In progress*) and *Done* (*Done*); every status can follow every other; entering a status of the *Done* group sets the closed date and leaving it clears it. Every issue type uses it until M4, whose workflow editor makes it the first workflow of the organization; stored statuses keep their IDs.
6. **Editors lock.** The layout editor takes an edit lock (ADR-0019) while open, like the workflow and theme editors.

## Consequences

- Own fields, layouts and per-type levels follow one inheritance rule people learn once.
- The settings registry stays a registry of single values; structures have their own tables and migrations.
- M4 replaces the starting workflow without a data migration of issues.

## Amendment, M3 WP-05: field levels per issue type

The lookup of decision 2 is `bb_core::lookup::nearest_row`: the one function that finds the row for a place and an issue type. The levels of fields per issue type use it today (`bb_policy::nearest_typed_rule`, which `nearest_rule` for the plain levels also calls); layouts (WP-09) use the same function.

- **Storage.** A level for one issue type is a row in `role_type_field_level` (the role's own) or `project_type_field_level` (set for a project), next to the tables of the levels for all types. The rows join the evaluation as `LevelRules::typed`; they are read together with the other levels of the roles at a place.
- **Reading.** `Evaluation::field_level_for(issue_type, field)` is the level of a field on an item of that type, `field_mask_for` the mask. A view that holds its issue type (`#[masked(item_type)]`) is masked with the mask of that type; a view that does not say its type gets, for each field, the lowest level the field has in any type, so a field that is hidden for some type is never shown by a view that does not say which type it is for.
- **Setting.** The route of the field permissions takes `?issue_type=`; taking a level for a type away is allowed at the organization too (the role's own level for all types always exists).
- **Names of own fields.** A screen that names fields (*Field permissions*, *Check access*, the move preview) lists an own field, and prints its name, only for a viewer who *may know the field*: they may see it in some issue type (any level above *Hidden*), or they have *Manage fields*, because whoever manages fields reads every definition on the *Fields* settings page anyway. For every other viewer the field's name, its ID and its rows are left out of the answer. Values are another matter: a value follows the level of its field, and *Manage fields* gives no right to read one. The rule is `may_know` in `bb-app` (`fields/catalogue.rs`); the move preview applies it per project of a row, so a field for a sub-project keeps its name for a viewer who may know it there.
- **Naming people in a field.** One rule, `may_name` in `bb-app` (`access/candidates.rs`), decides whether a person can be put into *Assigned to* or into an own field of people, and which people the list to choose from offers: the person exists, the viewer is shown people by name (*Assigned to* and *Project members* both show names, ADR-0022) or names themselves, and the person may see the item. A viewer who is shown others only as a role has no ID to name them by, so the ID of somebody else sent in a request is refused alike for a person who is there and one who is not. A value the field holds already, or that *Undo* puts back, skips only the access of the person named; the person still has to exist.
- **Names of tags.** The settings list of tags (*Organization settings › Labels & tags*) follows the same rule as the names of own fields and projects: a tag is listed, and can be changed, renamed or combined, only for a person who *may know the tag*: it has no limit, or it is limited to a project they see, or it is used in one. A tag only projects they cannot see know is not there for them (`resource.not_found`), and the answer never says that a hidden limit exists: a tag with limits only to hidden projects reads as for every project. Saving the projects of a tag replaces only the limits on projects the person sees; the limits on projects they cannot see stay, so a person never drops a limit they cannot see, and an empty list from such a person never makes a tag that is limited to hidden projects a tag for the whole organization. The rule is `known_tags` in `bb-app` (`tags/manage.rs`); the tests are `tag_01_the_settings_list_leaves_out_a_tag_only_a_hidden_project_knows`, `tag_01_saving_the_projects_keeps_the_limits_on_projects_the_person_cannot_see` and `tag_01_an_empty_list_from_such_a_person_does_not_make_the_tag_organization_wide`.

## Alternatives considered

- **Layouts as settings values (JSON in the registry):** no issue-type dimension, no typed validation, and every reader would parse JSON. Rejected.
- **Fixed statuses with no workflow model:** M4 would have to migrate every issue. Rejected.

## Enforced by

Review (the lookup function is the one place; its property test `cfg_01_a_layout_never_shows_a_field_the_role_cannot_see` lives in WP-09) and SEC-MATRIX for the levels per issue type.
