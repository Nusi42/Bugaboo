# WP-17 — The last screen fixes before the M4 review

**Exit wave B · Folder / branch:** `bugaboo.win5` / `WIN5`, beside WP-16
**Features:** WF-01…WF-06, ATT-01…ATT-07, NTF-01, NTF-02 (their screens).
**Source:** the final screen check of exit wave A, on the pictures in `<work root>/screenshots/M4/final`, taken from `WIN1` 9a7b66b7 after a green ci.

## Goal

The maintainer sees no screen of M4 that breaks UX design §1.1: no cut or broken text, labels level with their values, rows that do not move, one wording for a person the reader may not see.

## Decided by the maintainer

`main` moved to the green `WIN1` with these items open; they are done before the M4 review page and `m4-done`, beside WP-16.

## Owns

`web/app/src/**` (screens), `web/packages/ui/**`, `design/theme.kdl`, the screenshot specs, and the wording of the keys named below. WP-16 owns the API; take its generated client as it comes.

## Tasks

1. **The Details value column** is about 142 px wide and is the cause of several items below. Give the values the room the concept draws (the names column as narrow as its longest name), on computer and phone.
2. **Followers on one line** with its state and one button ("2 followers · You follow it" and *Stop following*), with no stray "·" at the start of a line, in every picture where the reader follows the issue.
3. **Names level with the first line of the value** also in the guest's view (`demo/07`): Priority with a long lock reason, and Followers.
4. **No cut text:** the customer "Alder Street Bakery" in `demo/04` and `demo/05` is shown in full (wrap, do not cut).
5. **A field's error does not move its row:** `workflows/15-name-taken-computer-*` (the group and *Create status* move about 22 px) and `16-publish-blocked-computer-*` (*Status group* below *Name of the status*).
6. **"Has rules" does not move the row** (`16-publish-blocked-computer-*`: the input and *Rules* shift by 4 to 12 px). This was a *Must* of the UX review.
7. **No word broken in the middle:** `M2/30-projects-*` breaks "Website" before its last letter; the short-name column lines up.
8. **Someone with the role:** "changed Assigned to from nobody to someone" and *Assigned to: Someone* in `demo/06` and `demo/07` follow the glossary (*Someone (Team member)*) and a plain sentence for an assignment.
9. **Disabled main buttons in Midnight** read as disabled, not as secondary buttons (`15-name-taken-computer-midnight`, `new-issue-*-phone-midnight`).
10. New pictures of every set into `.work/screenshots/M4/final` in your folder, and a look at each set in a light and a dark theme on computer and phone, all four themes for the issue page, the demo and the workflow editor.

## Depends on

`main` after exit wave A (WP-12 to WP-15). Works beside WP-16.
