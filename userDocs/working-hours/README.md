# The settings of working hours and time

Administrators set up working hours and time in **Organization settings › Work › Working hours**. The page needs the permission *Change organization settings*. Every change on it is written to the security log, and the page says so.

## The rules

**Working-time rules** is a preset: *Germany*, which follows the German Working Hours Act, or *No rules*, which checks nothing. Under the preset stand the numbers of the rules, each written as a length (`10h`, `1:30` or `90m`): the longest work day, when the first and the longer break are due and how long they are, the shortest break that counts, the longest stretch without a break, and the rest between two work days.

You can change any number. A number you changed shows the default value it can go back to, and **Reset to default** brings it back. A rule that is broken only gives a warning: it never stops anyone from recording what really happened.

## Keep work days for

Work days are kept at least this many years from their date. With the preset *Germany* the number cannot be below 2 years. Nothing is deleted by itself after that time: the page counts the work days that are older, for each person and in total, and **Delete them** deletes all of them or one person's, after a question in the page. The deletion takes their breaks and the history of their changes too, and is written to the security log. Whether an older work day is still needed is your organization's judgement.

## Totals only

While it is on, other people with the permission *See working hours of others* see each person's week total and nothing else, and nobody changes other people's work days.

## Billable

When it is on, a time entry has the box *Billable*. Switching it off hides the box and keeps what was marked.

## Who sees working hours

Everybody sees their own. The page lists the groups and people that hold a role with *See working hours of others* for the whole organization, with the way to *Roles and permissions*, where the role *HR* is given or taken away.

## Defaults of a project

In the settings of a project, **Defaults for time booked in this project** gives *Activity*, *Cost center* and every other field of time entries a default. Time booked on an issue starts with the value of the issue, else the value of its project, else the value of the project above. A sub-project shows **Set by the project above** until it sets its own value; **Reset to default** takes its own value away. The choices themselves are managed on the *Fields* page, like any own field.
