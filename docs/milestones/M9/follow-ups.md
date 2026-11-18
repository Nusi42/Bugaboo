# M9 follow-ups

Points found while building M9 that were accepted for later. Each names where it will be done. The IDs (`M9-F01` …) are what the review refers to. WP-01 uses F01 and up, WP-02 F10 and up, WP-03 F20 and up, WP-04 F30 and up, WP-05 F40 and up, WP-06 F50 and up, WP-07 F60 and up, WP-08 F70 and up, WP-09 F80 and up, WP-10 F90 and up, WP-11 F100 and up, WP-12 F110 and up, WP-13 F120 and up, WP-14 F130 and up, WP-15 F140 and up; rows found by the lead outside a package take F200 and up.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M9-F10 | A restore reads every kind of line of the replay list, but runs only the keys, AI app grants and passkeys taken back; removals and switch-offs are named in its report as not done again | Their replay needs *Remove a person* and *Switch off*, and the writer of the list is theirs | WP-06 (fills the two kinds in `bb-store/src/backup/restore.rs`, `replay_line`) |
| M9-F11 | A backup of the schedule that fails is written to the server log only: no alert on the status page and no notification to administrators (ADR-0046, decision 3) | The status page and its alerts are the screen's business | WP-11 |
| M9-F13 | How much the write-ahead log of SQLite grows while `VACUUM INTO` runs is not measured on the sample; the user docs say only that it grows and shrinks again | A measurement belongs to a quiet computer | WP-15 |
| M9-F14 | The row *Backups* of the keep table (ADR-0045) is not added yet | Whichever of WP-02 and WP-04 lands second adds it | WP-04 |
| M9-F15 | The list of backups (`GET /api/v1/admin/backups`) answers its files and its newest 50 records whole, with no `fields`, cursor or page size | The rotation keeps the files few and the records are capped by the code; the page that shows them decides what it needs | WP-11 (the page *Backups*) |
| M9-F40 | The files of *About* are a plain list of at most three, exempt from the lean-list rule as bounded by the code. If a fourth kind of file (for example the VEX statements) joins them, check that the list stays small by nature. | The list is bounded by the kinds of file; nothing grows with data. | WP-11 (the *About* screen), when it shows the files |
