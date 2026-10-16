# Changing many issues at once

In the list of issues of a project, tick the box in front of each issue. A line shows how many are ticked, with **Clear the ticks**, and a bar offers what you can do to all of them:

- **Change status or priority**
- **Move to a project**
- **Add a label**
- **Add a relation** (the kind of relation, and the other issue by its ID)
- **Delete**

Each opens a window. Fill in what you want and press **Show what changes**. Bugaboo does nothing yet: it lists each ticked issue and what would happen to it, with a sentence on top (*118 issues will change, 2 will be skipped*), ten issues at a time with **Show all**. Press **Change 118 issues** (the button says how many) only when that is what you want.

## Issues that are left alone

Bugaboo judges every issue again, with your own rights, in the preview and when you apply. An issue is **skipped**, and says why, when for example:

- it is not there for you, or you may not change it this way;
- it is like that already;
- it changed after the preview (somebody else was faster; nothing is overwritten);
- the other project does not offer its issue type, or does not use a field it has a value in (when moving);
- the relation is there already, or would relate an issue to itself.

A skipped issue never stops the others. After that a line says how many issues were changed and how many were skipped, and lists the skipped ones with the reason.

## A long change goes on in the background

More than 1,000 issues are changed in the background, a hundred at a time. The preview says so. You can go on working: a bar shows how far it is, and when it is done the line says how many were changed and how many were skipped. An access key cannot start a change to so many issues.

## Undo

**Add a label** and **Delete** can be taken back for a few seconds with **Undo**: the label comes off the issues again, and the deleted issues come out of the trash. The other changes are in the history of each issue, where you can change a value back.

## What happens to moved issues

An issue that moves gets an ID of the new project, and the old ID keeps working. It moves only into a project that offers its issue type and uses every own field it has a value in; tags it has stay only where the project allows them. You need the permission "Move issues" in both projects.

Each issue changed this way has **one entry in its history**, as if you had changed it by hand.
