# WP-13 — Nothing moves under the cursor

**Wave 3 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** UX-01 (calm screens), UX-02 (on touch). The maintainer's report of 2026-11-13: creating an issue makes the screen flicker; adding or removing a tag on an issue page (`/issues/BLOG-59`) makes everything reload and move. "A nice UI is always one where things don't move below your cursor. All of Bugaboo is bad at this, and it needs a full review pass."

## Goal

A change a person makes changes only what they changed. Nothing else on the screen moves, flashes, empties or redraws from scratch: not the rows around it, not the toolbar, not the page's scroll position, not the place under the pointer or the finger. Data that is read again in the background replaces what is shown in place, without grey shapes and without a jump. This holds on every screen, on a computer and on a phone, and a check keeps it so.

## Owns

The shared data layer of the browser app (`web/app/src/shared/**`: queries, invalidation after a change, optimistic updates, loading and grey shapes, Suspense and lazy parts), the shared UI parts that show loading or replace content (`web/packages/ui/src/**`, apart from the window phases of the fix in `bugaboo.win3`), the browser test support for measuring movement (`web/app/e2e/support/**`), the feature code each fix needs, the parts of `design/theme.kdl` they use, UX design §6 (a new rule) and architecture §11 (the measure).

## What is decided

- **The rule (UX design §6, new):** after a person's action, only the thing they changed may change its size or place, plus anything the action is meant to open, close, add or remove (a new row appears where it belongs; a closed window goes). Everything else stands still: no element of the page moves by even 1 px, and the scroll position does not change. A read in the background keeps what is shown until the new answer is there and then replaces it in place. Grey shapes appear only on a first read, and only after 0.3 s (§6.1), never on a read again. A list keeps its rows and their keys while it is read again. A part that may grow (a tag list, a note) grows downwards from where it is, never pushing what is above it.
- **The measure (architecture §11, new):** the browser's own record of layout shifts (`PerformanceObserver` with `layout-shift`, all entries including those after input), the boxes of named landmarks sampled every frame, and whether grey shapes or an empty list appeared. One helper in `web/app/e2e/support/` does this for any action; no spec has its own copy.
- **The full pass:** every screen of the route table (`places` plus the screens before sign-in and the timeline views, as the touch measure lists them) is measured for (1) a background read: every query of the screen is read again, and nothing may move or flash; (2) each in-place change the screen offers: tags, labels, status, priority, people, dates, own fields, the description, comments, relations, follow, time entries, the timer, settings switches and choices, the theme editor, the views and filters of a list, the timeline's changes. Each is listed once, in a table the test reads, with the landmarks that must stand still.
- **Fixes at the cause, shared first.** For example, keep the previous answer while reading again; update the cache from the answer of a change instead of invalidating whole screens; give a list stable keys; reserve the space of parts that load late; keep the focus and the scroll. No time limits and no retries.
- **On touch** the same rule holds, and nothing a finger is on may move between touch and release.
- **One way to scroll** (the maintainer, 2026-11-13, at `/issues/BLOG-5`: "sometimes the whole screen scrolls, sometimes a component has a scrollbar. We should be clean in this."): the page itself never scrolls. The app frame is the height of the window. Each pane (the main column, a side panel, a window) has exactly one scroll box: the box of its main content (the list's rows, the issue's sections, a form's fields). Whatever stands above that box (the title, the toolbar, filters, the counts, notices) gets smaller as the person scrolls the box, down to one compact line, and comes back when they scroll to the top, so the box gets the room. There is no scroll box inside a scroll box in the same direction, and no horizontal page scroll. The rule is written in UX design §6 with the movement rule, and the pass checks it on every screen: the document never scrolls, each pane has one vertical scroll box, and no box nests another in the same direction. On a phone the same model holds unless the browser's own bars need the page to scroll; the package says which, with the reason, in UX §6. The shrinking head follows the movement rule: it shrinks with the scroll, never jumps, and keeps the place under the pointer.

## Tasks

1. Write the rule in UX design §6 and the measure in architecture §11.
2. Build the measure helper and the table of in-place changes, and run them over every screen on desktop and phone. List every finding with its screen, its action, what moved, by how much, and its cause.
3. Fix the shared causes first, then each screen's own.
4. Make the full pass part of the browser tests, so a later change that moves something fails.
5. Take before and after recordings of the worst cases (creating an issue, adding and removing a tag, changing a status in a list, the timer).

## Acceptance tests (write first)

- `ux_01_adding_and_removing_a_tag_moves_nothing_else_on_the_issue_page` (computer and phone).
- `ux_01_creating_an_issue_moves_nothing_on_the_list_but_the_new_row` (computer and phone).
- `ux_01_a_read_in_the_background_moves_and_flashes_nothing_on_any_screen` (every screen of the route table).
- `ux_01_every_change_in_place_moves_nothing_else` (the table of changes, every screen).

## Checks

TEST-A11Y, TEST-CONCURRENT. The full pass joins the browser tests of the full run. A new check, if one fits, keeps every in-place change of a screen in the table (as TEST-A11Y keeps drags typed).

## Review

Code review with the SSOT/DRY/ODR gate: one measure, one table, one way to read again in place. The picture review compares the before and after recordings.

## Depends on

The window fix in `bugaboo.win3` (windows that do not jump when they open) lands first or beside it; this package does not change those window phases. WP-10 lands before it.
