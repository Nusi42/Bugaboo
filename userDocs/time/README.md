# Time

Bugaboo keeps two things apart on purpose. **Time on issues** says what you spent your time on, for accounting. **Working hours** say when your work day started and ended and when you had breaks. A timer never starts a work day by itself, and a work day never books time on an issue.

## The timer

A timer counts the time you work on an issue. There is one timer for each person, and it is kept on the server, so every device shows the same one. Closing the browser does not stop it; a timer started on the phone shows on the computer as soon as you come back to its window.

- **Start it:** press the play button beside *Time spent* on the issue page, press **Timer** in the bar of an issue on a phone, press `t` on an open issue, or open *Quick search* with `Ctrl` and `K` and choose **Start timer**. *Quick search* asks which issue when none is open.
- **See it:** the running timer is at the bottom of the sidebar, in the bar of a phone, and on the issue page. It shows the issue and how long it has run, in hours and minutes (`0:42`). It counts from the moment the server says it started, never from the clock of your device alone. On the issue page a row **Your timer** shows the clock and **Stop**. On a phone the bar of an open issue shows the running time in its **Timer** place; press it for **Stop**, **Switch issue**, **Change start time** and **Throw away**.
- **Use it:** press the timer to open **Stop**, **Switch issue**, **Change start time** and **Throw away**.

A timer that has run longer than the longest work day of your organization shows a warning on itself, and you get a notification.

### Stop

**Stop** opens a window with the sentence "You worked 42 minutes on "…"", the fields **Activity** and **Cost center** filled in from the issue, a place for what you did (optional) and, when your organization has switched it on, **Billable**. **Save time** books the time on the issue. **Throw away** stops the timer and saves nothing. A timer that ran for less than a minute saves nothing, and the window says so.

The fields start with the value of the issue. If the issue has none, they take the value of its project, then of the projects above it. Change them in the window when this time was different.

### Switch issue

**Switch issue** (or starting the timer on another issue while one runs) saves the time so far on the first issue, with its usual values and without a window, and goes on with the other issue. When a field that must be filled in has no usual value on the first issue, the **Stop** window opens first to ask for it. After a timer that ran under a minute Bugaboo says "Less than a minute on WEB-142, so no time was saved."

### Change start time

**Change start time** sets the day and the time of day the timer started. It cannot be in the future, and it cannot be before the end of your last time entry that was saved from a timer.

## Add time

Press **Add time** beside *Time spent* on the issue page to book time by hand: a date, a length, what you did, and the fields of the entry. A length can be typed as `1:30`, `1.5` or `90m`. It is at least a minute and at most a day.

The history of the issue says it as "Ada Admin booked 1h 20m · Development · 4100 Product development".

## The work day

The work day is at the bottom of the sidebar, and at the top of *Home* on a phone. The line says "Working since 8:02 · 6h 51m so far". Four buttons change the day:

- **Start work day** records the moment you press it as the start of your day.
- **Start break** and **End break** record your breaks.
- **End work day** records its end, and ends a break that runs.

Starting a timer before the work day asks "Start your work day too?". **Not now** is not asked again that day. Ending the work day while a timer runs asks "Stop the timer on WEB-142 too?". Neither starts or ends anything without your answer.

If you forgot to end a work day, a note says so with **Say when it ended**. The question "When did your work day on Thursday end?" takes the time of day and a reason, and the history of the day keeps the old times.

## A low window

A window lower than 720 pixels folds **My settings**, **Organization settings**, **Keyboard shortcuts** and **Narrow the menu** behind one button, **Settings and more**, so the timer and the work day fit. A person without saved searches sees no heading for them.

## The timesheet and working hours

- [The timesheet](timesheet.md): your week in one grid, typing lengths and times, warnings and reasons
- [Working hours of others](working-hours-of-others.md): looking at one other person's week
