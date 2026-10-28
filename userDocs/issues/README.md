# Issues

An **issue** is anything that is being tracked: a bug, a task, an idea. Every issue has a title, a description, a type, a status, a priority, the person who reported it and the people it is assigned to.

- [The issue page and the list of issues](issue-page.md): opening an issue, changing values where they stand, writing the description, and what happens when two people edit at once
- [Creating a new issue](new-issue.md): the dialog, `c`, and your draft
- [Issue IDs](issue-ids.md): `WEB-K7Q2X` or `WEB-142`, and what happens when a project changes from one to the other
- [Issue types](issue-types.md): Bug, Task, Feature and Goal, and how an administrator changes them
- [Sub-issues and relations](relations.md): parts of an issue, *blocks* and *duplicate of*, and the kinds of relation
- [Changing many issues at once](change-many.md): tick issues, read what would change, apply, and undo
- [Searching for issues](search.md): Simple and Advanced search, looking for words, lengths of time, saved searches and sharing, the list and Quick search
- [Limits of a search](search-limits.md): how many filters, words and steps a search may have
- [Comments, mentions, history and following](comments.md): writing to each other, naming people, reading what changed, and keeping track

## Who sees and changes what

- You see **every issue of a project** if your role has "See every issue of the project". Without it, for example as a guest, you see only the issues you reported. An issue you may not see is "Not found", exactly like an issue that does not exist.
- You can **create an issue** in a project if your role has "Create issues".
- You can **change an issue** if your role has "Change anybody's issues", or "Change the issues they reported" and you reported it. Each field also has a level for your role: you can change a field only where it says "Can change".
- You can **delete an issue into the trash** if your role has "Delete issues into the trash". It stays in the trash, with its ID, and "Undo" takes it out again.

Lists of issues come a page at a time. The count line says how many issues a search finds among the issues you may read, never more.

## Status

Each issue type has a workflow. At the start, Task and Goal use *Simple* (**Open**, **In progress**, **Done**), Bug uses *Bug* and Feature uses *Development*: see [Workflows](../workflows/README.md). The statuses belong to the groups *Not started*, *In progress* and *Done*. When an issue moves into *Done* it gets its closed date; when it leaves *Done* the date goes.

## Two people at once

If you change a value that somebody else changed after you opened the issue, Bugaboo tells you who changed it first and offers "Keep" their value or "Use mine"; nothing is overwritten unseen. While somebody is writing the description, others see "… is editing" and cannot change it at the same time: see [the issue page](issue-page.md).
