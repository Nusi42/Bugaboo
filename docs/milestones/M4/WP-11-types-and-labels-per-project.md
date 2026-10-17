# WP-11 — Issue types and labels per project, *Check access* per issue type

**Wave 3 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** ISS-02 (issue types per project), TAG-02 (labels per project), CFG-06 (settings in layers), SEC-03 (*Check access*); ADR-0027, ADR-0031; follow-ups M3-F22, M3-F122, M3-F124, M3-F44.

Tasks 9 and 10 of WP-04 move here: they need the model decided below, which WP-04 did not have.

## Goal

A project can offer only the issue types and labels it uses, and its Project lead manages them there. *Check access* tells what a person sees on an issue of one type.

## Decided by the lead

1. **Where a type or a label group is offered** is data, found the same way as layouts and workflows: one row per place, found with `bb_core::lookup::nearest_row`. No row means *offered everywhere*, so an upgraded database changes nothing. A row on a project counts for its sub-projects too, until a sub-project has its own row.
2. **Who manages them:** *Manage issue types, kinds of relation and layouts* and *Manage labels and tags* are judged at the place of the type or the label group. A type or group made for the whole organization still needs the permission for the whole organization. A Project lead manages those made in their projects.
3. **Moving an issue (M3-F124):** a type that is retired, or not offered in the target project, is named in the move preview as *"not used in {project}"*. The issue keeps its type, the same way a retired type is kept today. A retired type is never counted as missing.
4. **Labels:** a label group made in a project is offered there and below. Labels on an issue that moves out of their project stay on the issue and are shown, the same way as a retired label.
5. ***Check access* per type (M3-F44):** the report of `bb_policy` explains the level of a field for one issue type. The screen asks *"Which issue type?"* when the levels differ by type, and otherwise says nothing more.

## Owns

- The type and label-group places, in `bb-core`, `bb-app`, `bb-store` and `bb-api`.
- Migration **m0018**.
- The type and label screens in *Settings › Work*: the *Offered in* choice.
- The *Check access* screen and the policy explanation per type.
- The move preview text of a type.

## Tasks

1. The places of types and label groups (decisions 1, 2, 4), their API and their screens.
2. The move preview (decision 3).
3. *Check access* per issue type (decision 5).
4. Screenshots in four themes, computer and phone. The builder checks them against UX §1.1 before pushing.

## Acceptance tests (write first)

- `iss_02_a_project_offers_only_its_issue_types` (both databases).
- `iss_02_an_upgraded_database_offers_every_type_everywhere`.
- `sec_03_a_project_lead_manages_the_types_of_their_project_only`.
- `tag_02_a_label_group_of_a_project_is_offered_there_and_below`.
- `iss_02_a_retired_type_is_not_missing_on_a_move`.
- `sec_03_check_access_names_the_level_per_issue_type`.
- The permission matrix rows of the new routes.
- Playwright: a Project lead limits *Bug* to *Website*; *New issue* in another project does not offer it; *Check access* asks for the type.

## Checks

SEC-MATRIX and TEST-WORKFLOWS (the place lookup) grow.

## Depends on

WP-04 (workflows per type and place; this package reuses its lookup). It works beside WP-07 and WP-09; the type and label screens and *Check access* are not theirs.
