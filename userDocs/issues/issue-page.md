# The issue page and the list of issues

## The list of a project

Open a project and press "Issues of this project". The list is a grid in groups: "In progress", "Not started" and "Done", each with its number. Inside a group the newest issues come first, a page at a time ("Show more issues" brings the next page). Each line has a box you can tick, the status icon, the issue ID, the type, the title, the priority, the people it is assigned to and when it changed. A line is tinted by its status group and has a colored strip at the left for a high or critical priority; the Legend beside the "New issue" button says what the colors mean, and the icons say the same in words for people who use a reader. A whole line opens its issue. On a phone each line has two lines, in the same order. Ticked issues are kept until you clear them; changing many issues at once comes later. With `j` and `k` you move from title to title.

## Opening an issue

An issue opens in the **focus view** (it takes most of the page, the menu shrinks to icons and a slim list of the project stays on the left; `j` and `k` move through that list, "Back to list" or Esc returns) or in the **side panel** (it opens next to the list, so the list stays in view). Choose which one you like in *My settings › Appearance*, under "Open issues in". "Open in the side panel" and "Open full page" in the header switch from one to the other. The address of the focus view, for example `/issues/WEB-142`, is the one to share: the issue ID in the header copies it (the button is called *WEB-142, copy the address* and says *Address copied*).

On a phone an issue always takes the whole page, and a bar at the bottom offers "Status", "Comment" and "More".

## Changing a value where it stands

Click the title to change it. The big button under the title shows the status with its icon and color; press it to see the changes you can make and pick one. The **Details** are rows with the name at the left and the value at the right: pick another priority from its list, and choose who the issue is assigned to with "Change" (you can pick only people of the project). Each change shows at once and says "Saving…" and then "Saved · 10:42" where you made it. For the priority and the people, "Undo" stands in the same row for ten seconds; for the status and the title it is in the message at the bottom. On a phone the Details are closed behind "All details", which still says who the issue is assigned to and its priority.

A field you can see and may not change shows a lock and says so. A field you may not see is left out. Where Bugaboo may not tell you who somebody is, you see their role ("a Team member") or "Someone".

If somebody changed the same value a moment before you, nothing of theirs is overwritten: the page says, for example, "Ben changed Priority to Critical 2 minutes ago." and gives you two buttons, "Keep Ben's" and "Use mine".

## Writing the description

Press "Edit the description" (or `e`) to open the description for writing. Nothing is locked until you type the first letter, so just looking never stops anybody. While you write:

- Your text is kept as a **draft**, in your browser and, privately, on the server, until you save or throw it away (at most 30 days). If you close the window and come back, "Continue your draft from yesterday" is offered.
- If somebody else is editing the same description, you see "Ana is editing the description · since 2 min" and the buttons "Ask Ana to finish" and, if your role allows it, "Edit anyway". Ana is told within 15 seconds; with "Edit anyway" her unsaved text stays her draft.
- If your editing time runs out, somebody took over, or the description changed after you started, the page says "Not saved yet", keeps your text and offers "Compare and save": your text next to the newer one, paragraph by paragraph, with the words that differ marked (underlined and filled, so color is not the only sign), to choose what to keep.
- Every 30 seconds, and when you come back to the tab, Bugaboo checks whether the issue changed. If the description did, you see "Ben changed the description after you started" (the name is shown the way you may see people) with "Show changes", which marks the words that differ.

"Cancel" throws your text away.
