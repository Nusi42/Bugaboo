# The page Keep and delete rules

*Organization settings › Privacy › Keep and delete rules* is for people with the permission *Change organization settings*. Every person who signs in reads the same rules, without the controls, on *My settings › My data*.

## Two tables

- **Bugaboo deletes these by itself**: data that only serves a technical purpose, such as IP addresses, sign-ins, notifications and the trash. Once a day Bugaboo deletes what is past its period.
- **An administrator deletes these after their period**: business records, today working hours and time on issues. Bugaboo never deletes them by itself; it counts what is past the period, and an administrator deletes them.

Each row says how long the kind is kept, what happens then and why. [The table of the keep periods](keep-and-delete.md) lists the periods a new organization starts with.

## Change a keep period

Under the tables, each period that can be changed has a box. A period is the shortest time a kind is kept: a box does not take a number below what the law or the kind needs, and says which numbers it takes.

While you type a number, the line under the box says what it would do, before anything is saved:

- for a kind Bugaboo deletes by itself: *With this period, Bugaboo deletes 312 notifications within a day.*
- for a kind an administrator deletes: *With this period, 40 time entries are past it. Bugaboo deletes nothing by itself.*

Only **Save** changes the period. The tables show the new period at once, and the change is in the security log.

The period of backups is not changed here: it follows from how many daily and weekly backups are kept, on the page of backups.

## Delete them

For each kind an administrator deletes, a part of the page counts what is older than its period when you open the page, in total and for each person: *2 work days are older than the keep period of 2 years. These are the ones before 20 Nov 2024.*

- **Delete them** beside a person deletes that person's old records, after a question.
- **Delete them all** deletes everybody's, after a question.

What is deleted takes along everything that tells about it (breaks, the history of changes, notes). Afterwards the page says how many went: *2 work days were deleted.* Every deletion is written to the security log, with whose records and how many. The same part stands on *Organization settings › Working hours* for work days.

When the period changed or a day passed between the count and **Delete them**, nothing is deleted: the page counts again, so you delete only what you saw.
