# Releases

A **release** is a version of what a project makes, such as *2.5*. Issues name the releases they were found in and the release their fix is planned for, and the timeline and the calendar show a release on its *Release date*.

## Where a release lives

A release is created in one project and is offered in that project and in the projects inside it. A product with several parts plans one release for all of them by creating it in the project above them. The name of a release is unique within its project. An issue that moves out of the project keeps its release; the issue page says that the release is not offered there.

A release has a name, a *Release date* (the day it is planned for) and a status:

- **Planned**: the release is on its way.
- **Released**: the release was published. The day you marked it is kept as *Released on*.
- **Retired**: the release is no longer offered for new choices. The issues that name it keep it.

There is no start date: the issues of a release are its span.

## The page *Releases*

Every project has the page **Releases** (you reach it from the overview of the project). It lists the releases created in the project and those that reach it from above, with where they come from, in groups by status: **Planned**, **Released**, and **Retired**, which is folded until you press its name. Each release shows its *Release date*, the day it was released, and the number of issues planned for it that you may see; the number opens those issues as a search. You see the releases of the projects you see.

People who may **Manage releases** (at the start the Administrator and the Project lead) also see:

- **Create a release**: a window for the name and the *Release date*.
- **Change** on each release: the same window. It also tells how many issues are planned for the release and found in it, as far as you may see them.
- **Save the release** saves the name and the date. **Mark as released** keeps today as *Released on*. **Retire the release** stops offering it. A release that is released or retired goes back to *Planned* with **Back to Planned**; say why, and the reason is kept in the history.
- **Delete** puts a release that no issue names into the trash, with **Undo**. A release that issues name is not deleted: the window says "Issues name this release. Retire it instead." and names no issue.

Everybody else sees the page without these buttons.

## Found in release and Planned for release

The issue page has two rows in its Details: **Found in release**, which holds several releases (a bug can be seen in several), and **Planned for release**, which holds one (a bug is fixed in one). Press **Choose a release** or **Change** to open the list of the releases offered in the project, find one by its name and press **Choose**; every press saves at once and says it with **Undo**. A retired release is offered only while the issue names it. The form of a new issue has the same two fields where the layout of the project places them.

A search finds issues by these fields (`planned for release is 2.5`), a list or a timeline can be grouped by *Planned for release*, and the history names the releases that changed.
