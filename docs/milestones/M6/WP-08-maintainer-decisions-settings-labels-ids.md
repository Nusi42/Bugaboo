# WP-08 — The maintainer's decisions on settings screens, label squares and issue IDs

**Wave 2 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** PRIV-12 (*Who sees people*), SEC-04 (field permissions), SEC-02 (people and groups), TAG-02 (managed labels), ISS-01 (the issue ID). UX design §1.1, §5.2, §7; ADR-0026 (the contrast rules). Follow-ups M5-F173, M5-F135, M5-F149, M5-F137 (the issue ID in the code font), decided by the maintainer.

## Goal

The screens follow what the maintainer chose on the M5 review page: a group, a person and a role open the same way; *Who sees people* and *Field permissions* are one pattern; the label squares get clear colors; an issue ID always looks the same.

## Owns

`web/app/src/features/access/**` (groups, roles, people pages, *Who sees people*, *Field permissions*), `web/app/src/features/labels/` (the square), the rules of these parts and the label-square tokens and contrast lines in the theme files (after WP-02), the rule that sets the font of an issue ID, the user docs of these pages.

## What is decided (maintainer decision)

- **M5-F173:** a group opens as a page of its own, like a person and a role, with *Back* to the list; the panel beside the list is gone.
- **M5-F135:** one pattern for *Who sees people* and *Field permissions*: every role is a column, and the *Choose a role* box marks that role's column and shows the preview of what the role sees.
- **M5-F149:** the readability rule stays for every fill that carries text; the label squares carry none and get a looser rule of their own, so their colors are clear in every theme (no pale *Coral* next to *Pink* in the light themes, no brick red and dark olive in the dark ones).
- **M5-F137 (the ID):** an issue ID is in the code font everywhere: lists, the issue page, notifications, e-mails as far as the mail allows, history, *Quick search*, the timer.

## Tasks

1. **The group page** (M5-F173): its address, the page with its members and its roles, *Back*; the browser tests of groups follow.
2. **One pattern** (M5-F135): *Who sees people* takes the columns and the *Choose a role* box of *Field permissions*; the two share one part, written once. With it the small points of M5-F135: one style of the person picker; rows of roles with the arrow of a row that opens something; the origin text in sentence case; room for the focus ring under the last row of *Put people*; one place and font of the short name; no bullets in *Loses access*; the invitation link in the pictures; one way to show tiles or joined rows; a label for the list of a group's members; *Show more* for *Roles* on a computer; a group that does not look like a person; the sentence for zero people; more words in *More about this role*; the large gap of picture 22; help texts of one width.
3. **Label squares** (M5-F149): a `contrast` line of its own for fills without text (against the surface, 3:1 as for lines, and a least difference between neighbouring choices, a `distinct` rule), clearer values in each theme file, STY-CONTRAST green.
4. **The ID** (M5-F137): one part for an issue ID, used everywhere an ID is shown; the e-mail templates use the mono stack of the theme.

## Acceptance tests (write first)

- `sec_02_a_group_opens_as_a_page_like_a_person_and_a_role`.
- `priv_12_who_sees_people_and_field_permissions_choose_a_role_the_same_way`.
- STY-CONTRAST: the squares' line passes in every theme and mode; the squares of neighbouring choices differ by the `distinct` minimum.
- `iss_01_an_issue_id_is_in_the_code_font_everywhere` (a browser test over the list, the issue page, a notification and *Quick search*).
- The pictures of each changed screen in four themes on computer and phone.

## Checks

STY-CONTRAST (the new line), STY-PARTS, THEME-ONLY, TEST-A11Y.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: one part for the role columns of both pages, one part for an ID, one rule for fills without text. A UX reviewer looks at every picture against UX §1.1 and §7 and names the pictures looked at.

## Depends on

WP-02 merged (the theme files). Touches no file of WP-05, WP-06 or WP-07.
