# The board

The board shows the issues of a search as cards in columns, so you can see where the work stands and move a card from one column to the next.

## Show a search as a board

Open a search, a saved search or the list of a project. Above the list, set **Show as** to **Board**. Each column is a status at the start: statuses of the same name in different workflows are one column, so a board over a project and its sub-projects has one **In progress**. The columns come in the order of the workflow of the first project, and a column with no card stays.

Next to the name of a column stands how many cards it holds. The count is of the issues you may see; an issue you may not see is in no column and is not counted.

The cards of a column follow the sort of the search: at the start, the priority and then the last change. A board keeps no order of its own. The menu at the end of a column's head (**Order of In progress**) sorts the search by priority, by due date or by the last change.

**Refresh** reads the board again. It also reads again after you change something, and when the window gets the focus again.

## The card

A card shows the Issue ID, the title and the fields of the layout *Card* of its issue type, at most four. The layout is changed in the layouts of your organization; **Customize › Card** opens it for people who may change layouts. A card has the tint of its status group and the mark of its priority, like a row of the list. The title opens the issue.

A long column shows its first cards and **Show more cards**.

## Move a card

Move a card by dragging it to another column. This is the status change of its workflow, the same one the issue page makes:

- While you hold a card, the columns it cannot go to are drawn with a dashed outline and quiet words, and the reason stands in the column: "WEB-139's workflow has no status Testing", or "WEB-139 can't go from Open straight to Done".
- When the change asks for something (*Ask for*) or needs a field filled in (*Must be filled in*), the same window as on the issue page opens. **Cancel** leaves the card where it was.
- When it is done, Bugaboo says "WEB-139 moved to In progress", with **Undo**.
- When somebody else changed the issue after you read the board, Bugaboo asks as the issue page does: **Keep Ben's change** or **Use mine**.
- Dropping a card in the column it is in does nothing.

You can move a card with the keyboard or without dragging:

- **Move to …** on every card lists the columns. A column the card cannot go to is shown with its reason and cannot be chosen.
- Press `Tab` until the title of a card has the focus. `Space` picks the card up, `←` and `→` choose the column, `Enter` drops it and `Esc` puts it down. When no card is picked up, `↑` and `↓` move between the cards of a column, and `←` and `→` between columns.

## Card limits

A column can have a card limit. A column with a limit says "3 of 5" in its head. Going over the limit turns the title orange ("4 of 3"), and the head says so to screen readers, but Bugaboo never refuses a card because of a limit. The count is of the cards you see.

## Rows

A board can be split into rows by a field, for example by the person an issue is assigned to or by release. Each row has a head with its name and its count; press it to fold the row. The board shows up to 20 rows. When there are more, it says so.

## Customize the board

The person who created a saved search sees **Customize** above its board. The window sets:

- **Columns from:** *Status*, *Status group* (**Not started**, **In progress** and **Done**, which suits a board over very different workflows) or an own field that holds one choice. A column of *None* holds the issues without a value.
- **Rows:** no rows, or a field.
- **Card limits:** a number from 1 to 999 for each column; an empty box means no limit. Limits are set for the columns the board has now, so when you change the field of the columns, save that first.

**Save the board** keeps these with the saved search; people it is shared with open it with their own rights, and only its maker changes it. A board of a search that is not saved has a column for each status, no rows and no limits.

## The board on a phone

A phone shows one column at a time. The names of the columns are tabs, each with its count. Nothing is dragged: every card has **Move to …**, which does the same as a drag.
