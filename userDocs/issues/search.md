# Searching for issues

The search bar stands above a list of issues. It has two ways to write the same search. Switching between them never loses anything.

## Simple search

Each filter is a button, for example "Priority: at least high". Press it to change the filter, or press its ✕ to take it away. "Add filter" lists the fields you may read, the most used first, with a box to find the others. "Any of" joins filters with *or*: write them with "or" between them.

A filter that has no picker is a button with a dashed outline and its text. Press it to change it as text.

## Advanced search

Write the search as text, for example `status is Open and assignedTo is me`. The words are marked, suggestions come up where the cursor is, and a mistake is underlined and told in plain words below the box before anything is sent to the server. Press `Esc` to close the suggestions. Press "Help" for a sheet of the words with examples that go into the search with one press.

A field you may not read is told exactly like a field that does not exist.

Press `/` to go to the search bar.

### Looking for words

`contains` finds issues that have words in their title, description, comments or file names: `contains crash`, or `contains "login error"` for both words, which may be in different texts of the issue. End a word with `*` to find every word that starts like it: `contains notif*`. A word is a run of letters and digits; capitals do not matter, but letters and accents are looked at as they are written, so `Straße` and `strasse` are two words.

Issues that have the words in their **title** come first, then the others, newest first. Write `sort by` to choose another order. `title contains …` and `description contains …` look in that one text only. A field of your own that holds a long text is searched as `notes contains …`.

The limits of a search (how many filters, how long) are on [Limits of a search](search-limits.md).

### Lengths of time

A field that holds a length of time is written in hours and minutes: `2 hours`, `45 minutes` or `1 hour 30 minutes`. Only minutes and hours are used, so a search means the same for everyone. A bare number such as `5` is a mistake, and Bugaboo shows how to write it.

## Saving and sharing

"Save search" keeps the filters, with how the results are shown, under a name. At first only you can see it. A saved search has its own address, `/s/…`, and appears in the menu under "Saved searches": the ones you created, then the ones shared with you.

When you change an open saved search, "Save" (your own) or "Save a copy" (somebody else's) appears next to its name. Leaving without saving keeps the saved search as it was.

"Share" lists the groups and projects you may share with. Everyone sees only the issues and fields they are allowed to see. A project lead can also choose a saved search as the project's list.

## The list

The list under the search bar is the list of the project too: the same filters, the same columns. **Customize** chooses the columns, their order and width and the spacing; a column is sorted by pressing its name. **Group by** puts the issues in groups, for example by status group or by who they are assigned to, each with its count; a group folds with its arrow. The count line says how many issues the search finds among those you may read and how many are shown. Tick issues to change many at once.

A project can show a saved search as its list: its filters and columns are what you see when you open the project, until you change them.

## Quick search

Press `Ctrl` and `K` anywhere, or the button *Search* in the menu. Type words: the issues whose title has them come up under the box, with your saved searches and pages that match. The first result is ready: `Enter` opens it, the arrow keys choose another one, `Esc` closes the box. Typing an issue ID such as `WEB-142` opens that issue.

Quick search shows only what you may read, like every search.

## On a phone

The search bar is one button, "Filters (3)", that opens the filters in a sheet from the bottom. Advanced search is a text box in the same sheet.
