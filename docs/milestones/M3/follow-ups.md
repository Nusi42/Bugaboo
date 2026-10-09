# M3 follow-ups

Points found while building M3 that were accepted for later. Each names where it will be done. The IDs (`M3-F01` …) are what the review refers to. WP-01 uses F01 and up, WP-02 F10 and up, WP-03 F20 and up, WP-04 F30 and up, WP-05 F40 and up, WP-06 F50 and up, WP-07 F60 and up, WP-08 F70 and up, WP-09 F80 and up, WP-10 F90 and up, WP-11 F100 and up.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M3-F01 | The eleven M3 permissions are in the catalogue (WP-01) before any route asks for them, so each is named in `tools/checks/permission-exemptions.toml` | WP-01 adds every permission with its starting values in one change, so the starting roles and the role page are right from the start; the routes come with their packages | The package that adds a permission's first route removes its line: WP-03 (`see-issues`, `make-issues`, `change-issues`, `change-own-issues`, `delete-issues`, `manage-issue-types`), WP-04 (`edit-anyway`), WP-05 (`manage-fields`), WP-07 (`comment`), WP-08 (`move-issues`, `manage-labels`) |
