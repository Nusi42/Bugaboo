# The screen-reader pass: checklist

What a machine cannot tell about Bugaboo is heard with a screen reader: whether names make sense when they are read
out, whether the order of the reading follows the page, and whether a change is announced. This checklist is worked
through by hand, with NVDA (or Narrator) on Windows and the browser Bugaboo is tested in, on the sample organization
(`cargo xtask dev --sample`). It takes about an hour.

No person has made this pass yet: the first release goes without it (decided on 2026-11-21). The
[EN 301 549 report](en-301-549.md) marks every criterion that only a screen reader can tell *Not tested by a person*.
This checklist is the way to make the pass later.

## How to record the pass

Copy the table of each task below into the notes of the release, with the date, the screen reader and its version,
the browser and its version, and for each step *Yes*, *No* or *Partly* with what was heard. A *No* becomes a
follow-up with the step's number. When every step is *Yes*, the rows of the report that wait for this pass turn to
*Supports*, with the date of the pass as their evidence.

Start each task with the screen reader on and the browser window filling the screen. Use only the keyboard.

## 1. Sign in

| Step | Do | Hear |
|------|----|------|
| 1.1 | Open the address of Bugaboo. | The title of the page, then the heading *Sign in*. |
| 1.2 | Tab to the e-mail box and type an address. | "E-mail address, edit" (the name of the box, its role). |
| 1.3 | Tab to the password box, type a wrong password, press Enter. | The problem, read out at once, and the focus where it can be corrected. |
| 1.4 | Paste the right password from the clipboard, press Enter. | Pasting works (no test of memory, WCAG 3.3.8); the start page is announced by its title. |
| 1.5 | Sign out and sign in with a passkey, if the computer has one. | The passkey is offered by name; the dialog of the system is read. |

## 2. The issue list

| Step | Do | Hear |
|------|----|------|
| 2.1 | Press the key of *Skip to the content* (the first Tab). | The link is read and moves the focus to the main part. |
| 2.2 | Open the issues of *Website* from the menu. | The heading of the project; the list as a grid with its number of rows. |
| 2.3 | Move into the list and down three rows with the arrow keys. | Each row: its issue ID, title, status and priority in words, not only "row". |
| 2.4 | Move across the cells of a row. | The name of each column with its value. |
| 2.5 | Tick two rows with Space. | "Selected" for each, and the count of ticked issues. |
| 2.6 | Change the status of one row in place. | *Saving…*, then *Saved*, read without moving the focus. |
| 2.7 | Open *Group by* and choose *Status*. | The groups, each with its name and its number of issues. |

## 3. An issue and its comments

| Step | Do | Hear |
|------|----|------|
| 3.1 | Open an issue from the list with Enter. | The issue ID and title as the heading; the focus at the start of the issue. |
| 3.2 | Move through the headings (H in NVDA). | The parts of the issue in the order they are drawn: details, description, history. |
| 3.3 | Change the priority. | The choices with their names; *Saved* after the change. |
| 3.4 | Write a comment and save it. | The box of the comment by its name; *Saved*; the new comment in the history. |
| 3.5 | Change the status with the big status button. | The status changes offered, each in words; the new status announced. |
| 3.6 | Go back to the list. | The focus returns to the row of the issue. |

## 4. *New issue*

| Step | Do | Hear |
|------|----|------|
| 4.1 | Press `c`. | The window *New issue* by its name; the focus in the title box. |
| 4.2 | Press *Create* with an empty title. | The problem in words, read at once; the focus stays in the window. |
| 4.3 | Type a title, choose a type and press *Create*. | The window closes; the new issue is announced; the focus is on it or on what opened the window. |
| 4.4 | Open the window again and press Escape. | The window closes; the focus goes back to where it was. |

## 5. The timeline and its typed ways

| Step | Do | Hear |
|------|----|------|
| 5.1 | Open the issues of *Mobile app* and *Show as › Timeline*. | The timeline by its name; the rows of issues with their dates in words. |
| 5.2 | Move to a bar and open its card. | The issue, its start, its end, its person, and its plan warnings in sentences. |
| 5.3 | Use *Set dates…* to move a bar by a week. | The window by its name; the dates read in the region's form; the bar's new dates announced. |
| 5.4 | Use *Add dependency…* between two issues. | The dependency in words (*… must finish first*). |
| 5.5 | Move a bar with the keys of a bar (UX §4.7). | Each step of the move read out as dates. |
| 5.6 | Open the list of plan warnings and take a way out. | Each warning in a sentence; the way out and its result announced. |

## 6. A settings page

| Step | Do | Hear |
|------|----|------|
| 6.1 | Open *My settings › Appearance*. | The heading; the groups of choices by their names. |
| 6.2 | Choose *Always dark*, then a dark theme. | Each choice as a radio button with its state; the page changes without moving the focus. |
| 6.3 | Turn on *Larger text*. | The switch with its state; *Saved*. |
| 6.4 | Open *Organization settings › Keep and delete rules* and type a shorter period. | The box by its name with its unit; what the new period would delete, read once the typing stops. |
| 6.5 | Type a number out of bounds. | The problem in words, with the bounds. |

## 7. *My data*

| Step | Do | Hear |
|------|----|------|
| 7.1 | Open *My settings › My data*. | The heading; the tables of what is kept and how long, with their headers. |
| 7.2 | Ask for *Download my data*. | That the file is being made; later, that it is ready, without moving the focus. |
| 7.3 | Stay on the page while the file goes from being made to ready. | The file keeps every state in one place and shows one at a time (`StatesInPlace`): the state that is shown is un-hidden and the one before is hidden. The new state is said once, as words that were added, and neither the state before nor both are read out. |
| 7.4 | Download the file and open its README. | The README reads as a page with headings. |

## After the pass

Write the result into the notes of the release, change the rows of the [report](en-301-549.md) that waited for this
pass, and write a follow-up for every *No* and *Partly*.
