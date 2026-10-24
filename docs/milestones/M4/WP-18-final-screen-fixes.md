# WP-18 — The final screen fixes before the M4 review

**Exit wave B · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** WF-01…WF-06, ATT-01…ATT-07, NTF-01, NTF-02 (their screens).
**Source:** the final screen check, on the pictures taken from `main` 262adf7c after a green ci (in `.work/screenshots/M4/final` of `win1`).

## Goal

The maintainer sees no screen of M4 that breaks UX design §1.1. What the check found and is not listed here is already a follow-up with its own target (M4-F158 dates and the narrow menu's icons, M4-F159 rows without title or status, M4-F160 the *Check access* wording, M4-F161 empty fields as bare controls), and the review page names it.

## Owns

`web/app/src/**` (screens), `web/packages/ui/**`, `design/theme.kdl`, the screenshot specs and their support files, and the wording of the keys named below.

## Tasks

1. **Followers on one line** (WP-17 task 2, still open): on the issue page, in the files pictures and in `workflows/20`, `31`, *Stop following* drops under "1 follower · You follow it". The part `followers-state` lets the button wrap when there is no room; give the line the room, or a shorter layout, so count, state and button stay on one line in the Details column on computer and phone.
2. **"You follow it" level with the value column** (`demo/07`): `followers-state.words` keeps its start margin when no count stands before it.
3. **The field permissions table is not cut** (`M2/68-field-permissions-computer-*`): the fourth column shows only "C". It fits the page, or it shows plainly that it scrolls.
4. **The notification badge does not cover the bell** in the narrow menu (`demo/01`–`07` computer).
5. **File names never break at a hyphen** (`files-*-phone`, `files-progress-phone-*`, `files-undo-phone-*`, `demo/06-phone-*`): a name breaks only where there is no other way, like an issue ID.
6. **One sentence, not two** (`M2/35-move-preview-*`): "Nobody will gain or lose access or change role…" and "Nobody gains or loses access, and nobody changes role." say the same thing.
7. **The test field "Found in version"** (`web/app/e2e/support/workflows.ts`) uses a word the glossary does not allow, beside the built-in *Found in release*. Give it a glossary word.
8. **The date of an access key** (`M2/41-make-access-key-*`): the date box shows 23.01.2027 beside text that says January 22, 2027. Find out whether this is one day apart for a person (a time zone bug) or only how the box is drawn; fix a bug with a test that fails first.
9. **Phone pictures without blank space** under the bottom bar (`M2/04-appearance-phone`, `M2/53-role`, `M3/91-layouts-editor`).
10. **Small things:** disabled buttons in Dusk and Midnight get an edge that can be seen (now about 1.24:1 against the page); the key chips of *Keys now* and *Usual keys* line up (`M2/06`); the *Undo* toast does not cover the Details (`workflows/20`); "Starts with" is not in the code font (`M2/40`, `42`); the *Projects* heading of the More sheet starts where *Signed in as* starts (`M2/02`, `32`); an access key does not leave two letters alone on its last line (`M2/42`).
11. New pictures of every changed set into `.work/screenshots/M4/final` in your folder, and a look at each in a light and a dark theme on computer and phone; all four themes for the issue page and the demo.

## Depends on

`main` after WP-16 and WP-17.
