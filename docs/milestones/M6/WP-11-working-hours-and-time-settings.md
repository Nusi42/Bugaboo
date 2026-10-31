# WP-11 — The settings of working hours and time

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** TIME-09 (rules editable by administrators), TIME-10 (the keep period), TIME-12 (*Totals only*, the *HR* role on the role pages), TIME-13 (project defaults, required fields), TIME-02 (*Billable*), PRIV-09 (changes recorded). ADR-0018 (the settings registry draws the form), ADR-0035, UX design (*Organization settings › Working hours*, drawn by WP-01).

## Goal

An administrator sets up working hours and time in one page in plain words: the preset and its numbers, how long working hours are kept, *Totals only*, *Billable*; a project lead sets the default *Activity* and *Cost center* of a project; every change is recorded, and the page says so.

## Owns

`web/app/src/features/settings/working-hours*`, the project settings section of time defaults, the help texts of the working-hours settings, the user docs of these settings.

## What is decided (M6-Q7, Q8, Q10, Q11)

- *Organization settings › Working hours*: *Rules* (preset *Germany* or *No rules*, then each number with its plain sentence; a changed number marks the preset as changed and *Reset to default* brings it back), *Keep working hours for* (the minimum of ADR-0035, decision 7), the count of older work days with *Delete them* (all, or one person's, after a confirmation in the page; only for holders of *Change organization settings*), *Totals only* (with a sentence that says what others with *See working hours of others* then see), *Billable* (on or off).
- The page says that every change is in the security log, and who holds *See working hours of others* today (the groups, by name, with a link to *Roles and permissions*).
- The project settings: *Activity* and *Cost center* defaults of the project, inherited by sub-projects (*Set by the project above* when inherited, *Reset to default* when set here).
- *Activity* and *Cost center* themselves are managed on the *Fields* page like any own field (rename, choices, required); no second page for them.

## Tasks

1. **The page** from the registry (ADR-0018: one generic form; the rules' numbers with their units through the length helper of WP-03).
2. **The groups that hold the permission**, read through the role routes (no new route if an existing one answers it; otherwise one, declared with *See working hours of others* or *Manage roles*).
3. **The project defaults** through the own-field values of a project.
4. **Pictures** of the page and the project section in four themes on computer and phone, looked at before the push; the report names them.

## Acceptance tests (write first)

- `time_09_an_administrator_changes_the_longest_day_and_the_next_change_of_a_day_warns_by_it`.
- `time_10_the_keep_period_cannot_go_below_two_years_with_the_germany_preset`.
- `time_10_delete_them_asks_first_and_deletes_only_days_older_than_the_keep_period`.
- `time_12_turning_on_totals_only_is_in_the_security_log_and_others_see_totals_only`.
- `time_13_a_projects_cost_center_is_the_default_of_its_sub_projects_issues`.
- `time_02_billable_shows_in_stop_timer_only_while_the_organization_has_it_on`.
- TEST-A11Y on the page.

## Checks

ARCH-SETTINGS, TEST-A11Y, THEME-ONLY, TXT-TERMS, SEC-MATRIX (the settings routes under each role).

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the form from the registry, the rules' numbers from `bb_core::working_time`, no second list of the permission's holders. A security and privacy reviewer checks the log entries and who may open the page. A UX reviewer looks at every picture.

## Depends on

Wave 2 merged. Merges after WP-09 and WP-10.
