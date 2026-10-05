# WP-07 — Settings in layers and *My settings*

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** CFG-06, CFG-04, THM-02 (stored choice), PRJ-02 (the settings part); ADR-0018.

## Goal

Every setting of the registry can be read and changed at its levels — organization default, project, person — with *Reset to default* and settings an administrator locks, through one generic API and one generic settings form. *My settings* gets *Appearance*, *Region* and *Keyboard shortcuts*, stored for the person.

## Owns

`crates/bb-app/src/settings/**` (new use cases), `crates/bb-api/src/api/settings*` (new), `crates/bb-store/src/settings.rs` and migration `m0004`, `crates/bb-core/src/settings/**` (new kinds, new settings), `web/app/src/features/settings/**` and `web/app/src/features/my-settings/**` (new), the appearance source behind `useAppearance()` (WP-03), `userDocs/my-settings/*` (new).

## Tasks

1. **Resolution through the layers (CFG-06):** organization default → project (the nearest project up the tree that sets it, through WP-05's project table) → person; a value the organization locked wins over every lower layer. Each answer says where the value comes from (*default*, *organization*, *from ACME*, *your own*) and whether it is locked.
2. **Who may change what:** `changed_by` of the registry maps to the catalogue: *Administrators* → change organization settings; *Project leads* → change project settings on that project; *Each person* → only their own value. Every change asks `bb-policy`, writes its event, and settings that touch security also a security-log entry.
3. **Generic API:** list the settings of a level and topic with their metadata (type, bounds, list limits, help message key, levels, lockable) and resolved values; change a value; *Reset to default* (removes the value at that level); lock and unlock (organization level, lockable settings only). Lean answers and declared permissions as every route.
4. **Generic settings form (browser):** renders a topic from the metadata, one-line explanation under each name, a search box that filters by name and explanation, *Reset to default*, and *Set by your organization* (read-only, saying why, principle 10) for locked values; saves through WP-03's optimistic helper. Used on the *Organization settings* topic pages (mapping of topics from WP-01) and as a project section ready for the first project-level setting (none exists in M2; tested with test definitions of the registry, not shown on the project page yet).
5. **My settings (CFG-04, THM-02):** *Appearance* — *Match my device*, *Always light*, *Always dark*, one light and one dark theme picked from previews, spacing, *Less motion*, *Larger text*, focus view or side panel (UX §3.1); *Region* — time zone, date and number format, first day of the week; *Start page* — chosen from the page list of WP-03's quick-search sources (projects appear there once WP-08 adds its source); *Keyboard shortcuts* — see and change them (stored as one validated value; the browser registry of WP-03 checks the IDs). The settings are declared once in the registry; `appearance.theme` is replaced by the mode, light-theme and dark-theme settings, with stored values carried over by `m0004`.
6. **Connect the shell:** `useAppearance()` reads the person's resolved values (signed in) or the browser's (signed out), so the first frame keeps working as WP-03 left it (M0-F05).
7. **Personal data:** person-level values are linked to the person (marks on the value table as WP-01 decided; time zone is personal); the personal-data page is generated again.

## Acceptance tests (write first)

- `cfg_06_a_project_value_is_inherited_by_sub_projects_and_a_person_value_wins_for_the_person`, `cfg_06_a_locked_organization_value_wins_and_lower_values_are_refused`, `cfg_06_reset_to_default_removes_only_that_layer` (both databases).
- `cfg_06_only_administrators_change_organization_values_and_each_person_only_their_own`.
- `cfg_04_my_settings_are_kept_for_the_person_and_follow_them_to_another_browser`.
- Playwright: change theme mode, light and dark theme, *Less motion*, *Larger text*, time zone and a shortcut in *My settings*; sign in from a second browser and see the same; an administrator locks the theme and a person sees *Set by your organization*; four themes, phone, axe.

## Checks

ARCH-SETTINGS stays green with the new kinds; no new check. TEST-A11Y covers the new pages through `paths.ts`.

## Notes from WP-01

- **Personal data:** a person-level value is that person's data (`content`, `contract`, as today), and the person it belongs to gets its own field marked `identity`, `contract`: today it sits in `scope`, which is marked as not personal because it also holds project IDs ([12-security.md](../../architecture/12-security.md#personal-data-in-m2)).
- **Where the settings appear:** the topics of the registry map to the *Organization settings* menu as UX §5.1 lists them (`auth.sign-up-*` under People › People, `security.session-*`, `security.two-step-required` and `auth.access-key-*` under People › Sign-in rules, `appearance.*` under Look › Themes).
- The words *Reset to default* and *Set by your organization* are in the glossary.

## Depends on

WP-03 (shell, appearance source, page sources), WP-05 (policy, project table), WP-02 (lean answers).
