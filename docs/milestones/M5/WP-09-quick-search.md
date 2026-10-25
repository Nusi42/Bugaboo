# WP-09 — *Quick search*: issues, saved searches, projects, pages and actions

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** CFG-08. UX design §6.2 (with the M5 addition); M5-Q3; follow-up M2-F18.

## Goal

`Ctrl K` finds an issue by its ID or the words of its title, a saved search, a project or a settings page, and runs a few actions, always showing only what the person may see, and fast enough to follow typing.

## Owns

`web/app/src/features/shell/QuickSearch*`, `quick-search-*`, `QuickResults.tsx`; the sources that register through `addQuickSearchSource`; if needed a small route on the server for quick results (ask the lead first: the search routes of WP-04 should be enough).

## Tasks

1. **Issues:** an ID (both kinds, ADR-0021) opens the issue directly; words search the title through the search routes of WP-04 (`contains` limited to the title), the first ten results.
2. **Saved searches** (mine and shared with me), **projects** and **settings pages** that the person may see.
3. **Actions:** *Create issue*, *Go to …*, *Notification settings*, *Sign out* (M5-Q3).
4. **Typing:** a request only after a pause; one search counted per pause (M5-Q10); results in groups with their kind, issues first when the text looks like an ID.
5. **The first chunk:** move *Quick search* out of the first page's JavaScript if that page needs the room (M2-F18); PERF-BUNDLE says.
6. **Pictures** of *Quick search* in the four themes on computer and phone, looked at before you push.

## Acceptance tests (write first)

- `cfg_08_an_id_opens_the_issue`, `cfg_08_words_find_issues_by_title`.
- `cfg_08_quick_search_never_shows_what_the_person_may_not_see` (a hidden project, a hidden issue, a saved search they may not see).
- `cfg_08_typing_counts_one_search_per_pause`.
- Keyboard and screen reader: the results are a list box, the arrow keys move, `Enter` opens (TEST-A11Y).

## Depends on

WP-04 and WP-06 merged. Runs beside WP-07 and WP-08.
