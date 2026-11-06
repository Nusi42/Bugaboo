# Who sees working hours, and how long they are kept

Working hours and time entries are sensitive data about employees. Bugaboo treats them that way.

## Who sees them

- **You** see and record your own working hours and time entries, always.
- **Other people** need the permission *See working hours of others*. The starting role *HR* holds it and *Change working hours of others*; nobody has *HR* at the start, and the role *Administrator* does not include either permission. An administrator gives *HR* to a group, for example *Payroll*, in *Roles and permissions*.
- The one place that shows the working hours of another person is [Working hours of others](../time/working-hours-of-others.md): one person at a time, in the rows of the work day, without time on issues.
- With **Totals only** on, that page shows each person's total of the week and nothing else, for everybody, and nobody changes the days of other people while it is on.
- A holder of the permission sees the names of the people there, whatever *Who sees people* says elsewhere.
- No other page, total, count, history, notification, e-mail, export or search tells you about the working hours of another person.

## What Bugaboo never does

- It never orders, ranks or compares people by their hours, their booked time or their warnings. A list of people stays in the order of their names.
- It never gives a score to a person.
- No AI tool reads working hours or the time entries of another person.

## Changes are recorded

Every change of the permissions that give access to working hours, and every change of the settings of working hours (the preset, a rule, the keep period, *Totals only*, *Billable*), is written to the security log. A change of another person's work day always carries a reason, and the person is told about it with that reason.

## How long they are kept

Work days are kept at least the number of years you set under **Keep work days for** (see [the numbers](numbers.md) for the start value and the least with the preset *Germany*). Nothing is deleted by itself. The settings page counts the work days that are older, for each person and in total, and an administrator deletes them explicitly, after a question. The deletion takes their breaks and the history of their changes too.
