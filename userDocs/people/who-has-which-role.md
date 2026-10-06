# Who has which role

*Who has which role* is the list of the people and groups with a role, and where it counts. There is one for the whole organization, and one for each project.

- **The whole organization**: *Organization settings › Roles & permissions*, below the roles. A role given here counts in every project.
- **A project**: *Project settings › Who has which role*, at `/p/WEB/settings/roles`. A project lead sees it for their own projects.

## Where it counts

A row on a project says where it counts:

- **Only this project**
- **This project and its sub-projects**: it also counts in the projects inside, including ones made later

A row for the whole organization counts everywhere. Giving a role to a group gives it to everybody in the group.

## Rows that come from above

A project's list shows the rows of the project first, and then the rows that reach it from above: from a parent project (*from Acme*), from the whole organization, or from a project you cannot see (*from a project you can't see*; it is not named).

A row that comes from above is not taken away here. "Change for this project" starts a new row in this project for the same person or group, for example a different role, or No access.

## Giving a role

Under "Give a role": choose **a person** or **a group** and search for them by name, choose the role, and, in a project, where it counts. The people and groups you picked last come first. The form never offers or searches e-mail addresses.

You can give a role only if you hold every permission of it in that place yourself. A person or group cannot have two rows for the same role in the same project, and not a role and No access in one project: the sentence says which row is in the way.

## No access

"No access" is a choice of the role in a project. The person or group sees and can do nothing in that project and its sub-projects. It wins over any role from above and over a role they have through a group on the same project. A role given in a sub-project further down still counts there.

People who decide who has which role *above* the project keep seeing its name here, so they can take a No access away again. They see nothing in the project.

## Taking a row away

"Take away" happens at once and offers **Undo** for 10 seconds, which gives the row again. A question comes first when taking the row away would take away your own way to manage roles or to decide who has which role: the row is yours, or it is the row of a group you are in. Bugaboo refuses a change that would leave nobody who can manage roles for the whole organization, with a sentence that says why.
