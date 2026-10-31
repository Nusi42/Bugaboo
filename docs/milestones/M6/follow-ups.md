# M6 follow-ups

Points found while building M6 that were accepted for later. Each names where it will be done. The IDs (`M6-F01` …) are what the review refers to. WP-01 uses F01 and up, WP-02 F10 and up, WP-03 F20 and up, WP-04 F30 and up, WP-05 F40 and up, WP-06 F50 and up, WP-07 F60 and up, WP-08 F70 and up, WP-09 F80 and up, WP-10 F90 and up, WP-11 F100 and up, WP-12 F110 and up, the exit visual pass and the exit waves F120 and up.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M6-F10 | A theme file may set the shadow as well as colors, because the shape of the shadow differs between the themes (a ring in Midnight, a deeper drop in the dark themes); one shape in the base with a color per theme would make theme files colors only | One shape changes the look of Dusk and Midnight; the split was to change nothing on screen (ADR-0026, update M6) | The theme editor (M7, THM-03/04), as a decision of the maintainer |
| M6-F12 | The showcase pictures of long lists are not repeatable (random issue IDs, a varying number of loaded rows), so 3 of 488 differed between two runs without a change of look | Only the data differs; the colors are the same | The exit visual pass: make those pictures repeatable or mask the varying parts |
| M6-F13 | `Theme.kind` in the theme reader is a string checked against `KINDS`, not an enum | Old code; the theme files kept its shape | The next package that changes the theme reader (the theme editor, M7) |
| M6-F14 | No picture or browser test covers more contrast with a device in dark mode on a theme other than Dusk; that is how Midnight showed Dusk's colors there | The unit test `thm_06_…` now guards the generated sheet | The exit visual pass, or an accessibility test for that mode |
| M6-F15 | In the theme reader, `is_property` still writes the rule of a name itself instead of calling `is_word`, and `sty_contrast.rs` works out the place of a finding twice | Small; found by the second review of the theme files | The next package that changes the theme reader or STY-CONTRAST |
