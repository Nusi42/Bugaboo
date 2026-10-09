# WP-05 — Own fields: types, choices, rules, looks, permissions

**Wave 2 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** CF-01, CF-03, CF-04, CF-06, CF-07, CF-08 (issues and projects), SEC-04 (own fields, per issue type); UX design §5.2, §5.3, §7.1; ADR-0006, ADR-0025, ADR-0027.

## Goal

An administrator adds the fields the organization needs (*Severity*, *Platforms*, *Customer*) in plain dialogs; each field has its own permissions per role, and people fill them in on issues and projects exactly like the built-in fields.

## Owns

`crates/bb-core/src/fields.rs` (the move from `Field` to `FieldKey`, WP-01's design) and `crates/bb-core/src/own_fields*.rs` (new: types, values, rules, looks), `crates/bb-app/src/fields/**` (new), `crates/bb-store/src/fields.rs` (new) and migration `m0009` (`field`, `field_option`, `field_value`, per-type field levels), the field-level code in `bb-policy` (own fields and issue-type rows), `crates/bb-api/src/api/fields*` (new), `web/app/src/features/fields/**` (new: *Organization settings › Fields*), the fields slot of the issue page and of the project overview (WP-06's slots), the field parts in `design/theme.kdl`, `userDocs/fields/*` (new).

## Tasks

1. **Field key (CF-06):** `FieldKey::BuiltIn(Field) | FieldKey::Own(FieldId)` everywhere a field is named: levels, masks, the matrix of *Field permissions*, history, `fields=` in the API, SEC-MATRIX. The built-in catalogue stays one enum; no list of own fields is written twice.
2. **Types (CF-01):** text, long text (Markdown via WP-02, locked while edited via WP-04), whole number, decimal, yes/no, web link, date, date and time, duration, pick one, pick several, person, people, group, issue. Values in `field_value` with one typed column per kind (ADR-0006); several rows for several values.
3. **Scope and rules (CF-03):** the whole organization, a project and its sub-projects, or some issue types; required, default value, help text; range, length, pattern (a pattern is checked with a linear-time engine and a length cap). The same rules are sent to the browser for instant feedback; the server decides.
4. **Choices and looks (CF-04, CF-07):** add, reorder (drag and buttons), color as a token `cat.1…12` and an icon per choice, *Combine with another choice* (moves every value, in the history of every issue it touches), *Retire* (gone from new choices, kept on old issues); a field's look: badge, pill, plain text, progress bar, avatar, with a live preview in all four themes; *Retire* a whole field without losing data.
5. **Permissions (CF-06, SEC-04):** each own field joins *Field permissions* with the starting levels of WP-01; levels per issue type under *More options* (ADR-0027 lookup order). *Manage fields* to change definitions.
6. **Where it's used:** the field page lists projects and issue types that use it before a change ("Used in 4 projects").
7. **Own fields on projects (CF-08):** the same fields with scope *Projects*, shown on the project overview.

## Acceptance tests (write first)

- `cf_01_every_field_type_stores_and_returns_its_value` (both databases, property test per type), `cf_03_a_required_field_and_a_rule_refuse_a_wrong_value_with_a_sentence`, `cf_03_a_field_for_one_project_is_not_offered_elsewhere`.
- `cf_06_an_own_field_follows_its_level_per_role_and_per_issue_type`, `cf_06_a_hidden_own_field_never_leaves_in_any_answer_or_history`.
- `cf_07_combining_two_choices_moves_every_value_and_keeps_a_record`, `cf_07_a_retired_choice_stays_on_old_issues_only`.
- `cf_04_every_look_is_readable_in_all_four_themes` (STY-CONTRAST over the field looks).
- Playwright: make *Severity* (pick one, four choices with colors), *Platforms* (pick several), *Customer* (text); fill them on an issue; see *Severity* hidden for a Guest; four themes, axe.

## Checks

None new. SEC-MATRIX adds own fields; DATA-MARKS covers `field_value` (its text is personal unless the field says otherwise).

## Depends on

WP-01 (ADR-0027, the field-key design, starting levels), WP-02 (long text), WP-03 (issues, issue types), WP-04 (versions, locks), WP-06 (the slots it fills).
