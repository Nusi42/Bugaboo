# Time on issues

Time on issues is the time you spent on an issue, booked for accounting. You book it with the timer, with **Add time** on the issue page, or by typing a length into a cell of [the timesheet](timesheet.md). Working hours are a different thing: see [Time](README.md).

## Time entries

A time entry has a person, an issue, a date, a length, an optional note, and the fields of time entries. A length is at least a minute and at most a day, and you can type it as `1:30`, `1.5` or `90m`. The note is plain text. You change or delete your own entries; nobody changes the entries of another person.

You need the permission *Book time* on the project of the issue. The roles from *Contributor* up have it; *Viewer* and *Guest* do not. A date holds only so many entries (see [the numbers](../working-hours/numbers.md)).

An entry saved from a timer also keeps the moment the timer started, and its date is your day of that moment.

## Activity and Cost center

*Activity* (*Development*, *Testing*, *Documentation*, *Meetings*, *Support*) and *Cost center* (a number and a name, like *4100 Product development*) are fields of time entries. An administrator renames, changes or retires them like any other field, and can set either one as required; an entry without a required value is refused with a sentence that names the field.

A new entry starts with the value of the issue. If the issue has none, it takes the value of its project, then of the project above. You set the value of an issue in its details, under *Defaults for time booked on this issue*, and the value of a project in the settings of the project.

When your organization has switched on **Billable**, an entry has a box for time that can be charged to a customer. Switching it off hides the box and keeps what was marked.

## Estimated time, Time left and Time spent

- *Estimated time* and *Time left* are lengths that you type on the issue like any field.
- *Time spent* is the total of the time entries of the issue. You read it; you never type it.
- While nobody has typed *Time left*, it is *Estimated time* minus *Time spent*, and never below zero. The page shows such a value in a quiet style, with the words "Worked out". A value that somebody typed counts until it is cleared.

An issue with sub-issues shows its own numbers and the totals with every issue below it. A project shows the totals of its issues and of its sub-projects.

A total counts only the issues you can see and the fields you may read. It never tells you about an issue or a field that is hidden from you.

## In searches

*Time spent*, *Estimated time* and *Time left* can be used in a search to filter, sort or show as a column. See [Searching for issues](../issues/search.md).
