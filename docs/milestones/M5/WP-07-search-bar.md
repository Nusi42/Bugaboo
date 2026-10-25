# WP-07 — The search bar: Simple and Advanced, suggestions, mistakes, *Save search* and *Share*

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** QRY-01, QRY-02, QRY-03 (on screen), QRY-06 (on screen). UX design §2.1, §4.3 (all of it, with the M5 additions), §6.2 (`/`); ADR-0005 amendment M5, ADR-0009, ADR-0034; follow-ups M3-F46, M3-F62.

## Goal

One search bar above every list. Simple search and Advanced search show the same search, switching never loses anything, mistakes are explained in plain words before the server is asked, and a search can be saved and shared.

## Owns

`web/app/src/features/search/**`; the address format of a search (`?s=` with the canonical text, read and written in one place; `/search` for the *Search* page, added to UX §2.1); the search keys of the browser catalog; the saved-search screens (*Saved searches* in the sidebar, *Save search*, the *Share* window, the help sheet); the date range editor (M3-F46) and finding an issue by its title for an *Issue* field (M3-F62); the screenshot specs of these screens.

## Tasks

1. **The search in the address:** the canonical text in `s`; the view parameters next to it (UX §2.1). Back and forward work; a shared address opens the same search for anyone who may see it.
2. **Simple search:** filter buttons from the tree (through the WebAssembly module of WP-02): one per filter, ✕ to remove, *+ Add filter* from the person's readable fields (most used first, with a search box), *Any of* for OR, and the dashed text button for what has no picker. Pickers for choice fields, people, projects, dates (the date range editor) and issues (by ID or title, through the search route).
3. **Advanced search:** a text box with the words marked, suggestions at the cursor (values only the server knows are asked for, WP-02 task 5), mistakes underlined with the checker's message, the last results kept dimmed until the search is fixed.
4. **Switching:** Simple ⇄ Advanced edits one tree; a test types texts in Advanced, switches to Simple and back, and gets the same canonical text.
5. **Save, change, share:** *Save search*, *Save* or *Save a copy* next to an open saved search's name, the *Share* window with only the groups and projects the person may share with, and *Use as the project's list* for those who may.
6. **The help sheet** (UX §4.3).
7. **On a phone:** *Filters (n)* opens the sheet (UX §4.3).
8. **Mistakes as text** (WP-02 review): a mistake's arguments are the person's own text; render them as text, never as HTML.
9. **Pictures:** every new screen in the four themes on computer and phone, looked at against UX §1.1 before you push; the report names the pictures you looked at.

## Acceptance tests (write first)

- `qry_02_switching_between_simple_and_advanced_keeps_the_search` (browser, a table of texts).
- `qry_03_a_mistake_shows_before_the_server_is_asked_and_matches_the_server` (the same key and range from the module and from the route).
- `qry_05_a_hidden_field_in_advanced_search_says_there_is_no_such_field`.
- `qry_06_a_saved_search_opens_at_its_address_with_its_columns`, `qry_06_share_lists_only_what_the_person_may_share_with`.
- Keyboard: `/` focuses the bar; every control works with the keyboard and a screen reader (TEST-A11Y).

## Depends on

WP-04 and WP-06 merged. Runs beside WP-08 and WP-09; WP-08 reads the address through this package's one function.
