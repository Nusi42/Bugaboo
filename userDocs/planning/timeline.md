# The timeline

The timeline shows the issues of a search as bars over the days they run, so you can see what happens when, what waits for what, and what has no dates yet. Some people call such a chart a Gantt chart.

## Show a search as a timeline

Open a search, a saved search or the list of a project. Above the list, set **Show as** to **Timeline**. The tools above the chart set how it looks:

- **Group by** puts the rows into groups you can fold: by goal (the start), project, person, status, priority, or an own field with choices. Press the head of a group to fold it or open it again. Issues without a value are in a group of their own: **No goal**, **No person**, or **No** and the name of the field. The head of a goal shows its Issue ID and its title.
- **Bars start at** and **Bars end at** choose the two date fields the bars use: *Start date* and *Due date*, or two own date fields of your organization.
- **Plan by effort** lets the timeline work out the other end of a bar from its estimate when an issue has only one date. The days that are worked out are shown, never saved.
- **Zoom** sets how many days one column holds: **Days**, **Weeks**, **Months** or **Quarters**.
- **Earlier** and **Later** step through time; **Today** goes back to today.

Everything you set is part of the view. **Save search** keeps it with the search, and the address of the page holds it too, so an address you share opens the same timeline.

## What the chart shows

- On the left, each row has the status icon, the Issue ID and the title of its issue.
- The scale at the top names the days, weeks, months or quarters. Weekends are shaded at the zoom of days and of weeks.
- **Today** is a line in the accent color.
- A **bar** runs from its first day to its last day. It has the tint of its status group and the mark of its priority at its start, like a row of the list, and a darker part shows how much of it is done. Its Issue ID stands after its end.
- An issue with one date is a bar of one day, unless *Plan by effort* works out the other date.
- An **arrow** goes from an issue that must finish first to the one that waits for it. These are the relations of the kind your organization gave the date effect *Must finish first* (at the start, *blocks*).
- A **goal** shows a bracket over the days of its sub-issues, in the head of its group, and a **diamond** on its *Due date*.
- The Legend under the tools explains the bracket, the diamond and the Today line; the Legend beside the toolbar explains the tints and the marks of priority.

Issues you may not see are not on the timeline at all: no bar, no arrow, and they are not counted.

## Change dates by dragging

- **Move a bar:** drag it. Both its dates move by the same number of days.
- **Change one date:** select the bar, then drag the handle at its start (its start date) or at its end (its end date: *Due date*, or the own date field the view ends at).
- **Add a dependency:** select the bar, then drag the dot after its end onto another bar. The bar you start from must finish first; the other one waits for it. You may do this when you may change one of the two issues; when you may not, Bugaboo says why.

While you drag, only the bar moves and nothing is saved. When you let go, the dates are saved, and Bugaboo says what happened and which dates the issue now has, for example "WEB-280 moved 1 week later, now from 10 Nov 2026 to 14 Nov 2026", with **Undo**. **Undo** puts the dates back as they were. When Bugaboo cannot save a change, it says why.

When *Plan by effort* worked out one end of a bar and you move that end, the day the bar showed there becomes a date of the issue; the sentence names both dates.

A bar of an issue whose dates you may not change cannot be dragged. Point at it, or press an arrow key on it, and Bugaboo says why: "You may not change the dates of WEB-280."

## Change dates with the keyboard

Every drag has a key or a button that does the same:

- Press `Tab` until a bar has the focus. Press `Enter` or `Space` to select it.
- `←` and `→` move the bar a day earlier or later; with `Shift`, a week.
- `Alt` with `←` or `→` moves its end date a day; `Alt` and `Shift` with `←` or `→` move its start date.
- Presses one after the other are one change: Bugaboo saves it when you let go of the keys for a moment, or when you leave the bar, and offers one **Undo** for it.
- The card of the selected bar, beside the chart, has **Set dates…**, **Add dependency…** and **Open**.
  - **Set dates…** opens a window with the two date fields. Type the days in your date format; an empty box takes a date away.
  - **Add dependency…** opens a window where you choose which issue waits and find the other issue by its ID or by words of its title.
  - **Open** opens the issue.

## Not scheduled yet

Beside the chart, **Not scheduled yet** lists the issues of the search that have no dates (or none you may see). Drag one onto the chart: it starts on the day you let it go. Or press **Set dates…** next to it.
