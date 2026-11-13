# WP-11 — Touch everywhere: targets and swipe actions

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** UX-02. UX design §3.3 (phone) and §1.1; follow-ups M5-F158, M6-F121.

## Goal

Every screen works with a finger: every control has a target of at least 44 px on touch, the *Compact* tick boxes included, phone form footers keep their gaps, and list rows have swipe actions that are also in *More*. Phone pictures of every screen prove it.

## Owns

The touch variants of the tokens and parts in `design/theme.kdl` (the `touch` choice of the device, ADR-0026 update M3 WP-16), the swipe actions on list rows in `web/packages/ui/src/` (one component, used by the issue list; notifications and *My agents* keep their actions as buttons and have no swipe, decided by the maintainer, M8-F116; the saved searches wait for a later milestone, M8-F101) and their entries in *More*, the fixes of M5-F158 (the *Compact* tick boxes) and M6-F121 (the gaps of phone form footers), a check of the size of targets in the browser tests (`web/app/e2e/support/`), the phone pictures of every screen.

## What is decided

- **44 px on touch, 36 px with a mouse** (the UX concept): through the `touch` choice of the theme file, never a size written in a component. *Compact* changes spacing, never a touch target below 44 px.
- **Swipe actions** on list rows: one component, with at most two actions to each side, chosen per list (the issue list: *Follow* and *Change status or priority…*, decided by the maintainer, M8-F102; notifications and *My agents* show every action as a button and have no swipe, decided by the maintainer, M8-F116; only a row that hides actions swipes), each also in the row's *More*, so nothing depends on a swipe (TEST-A11Y). *Less motion* shows the action without the slide.
- **The new screens of M8** (work times, project members, the theme editor, the phone timeline) are covered as well: WP-11 runs in wave 3 for that reason.
- **Phone pictures of every screen** (04-milestones, M8 checks): one per screen in the four themes, named so the UX review can find each.

## Tasks

1. Write the acceptance tests below.
2. Measure every control on every screen at the phone width with touch, and list those below 44 px.
3. Fix each through the theme file; M5-F158 and M6-F121.
4. The swipe component and its use on the four lists, with the entries in *More*.
5. The phone pictures of every screen in the four themes, looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `ux_02_every_control_has_a_touch_target_of_at_least_44_px_on_a_phone` (browser, every screen of the route table).
- `ux_02_compact_tick_boxes_keep_44_px_on_touch` (browser; M5-F158).
- `ux_02_phone_form_footers_keep_their_gaps` (browser; M6-F121).
- `ux_02_a_swipe_on_a_list_row_offers_the_same_actions_as_more` (browser, phone).
- `ux_02_less_motion_shows_swipe_actions_without_sliding` (browser, phone).

## Checks

TEST-A11Y, THEME-ONLY, STY-TOKENS, STY-PARTS, LINT-CSS, TXT-LITERAL, TXT-TERMS, ARCH-WEB-FEATURES, LINT-TS.

## Review

Code review with the SSOT/DRY/ODR gate: one swipe component, sizes only from the touch tokens. A **UX review** of the phone pictures of every screen in the four themes against UX design §3.3, §1.1 and §7.

## Depends on

Wave 2 merged (the screens of WP-07 and WP-08 exist). Lands after WP-09 and WP-10, so their screens are measured too; a finding on them goes back to that package.
