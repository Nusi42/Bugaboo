# Field permissions

For each role you choose, for every field, one of three levels:

- **Hidden**: the field does not exist for that role. It is not in issues, searches, e-mails, maps or the assistant.
- **Can see**: the field is shown.
- **Can change**: the field is shown and can be changed, where the person can change the item at all.

The fields are the built-in ones: Title, Description, Status, Priority, Assigned to, Reported by, Due date, Labels, Tags, Planned for release, Found in release, Estimated time, Time left, Time spent, Activity and Cost center. Fields you create yourself join the table later.

## For the whole organization

*Field permissions* in *Organization settings*, below "People", shows the roles as columns and the fields as rows. Each cell is a list of the three levels. The level you pick is the role's own level; it counts everywhere unless a project says otherwise.

## For one project

On *Project settings*, open the tab *Field permissions*. Change a cell to set a level for this project and the projects inside it. The cell then says *Set for this project* and offers **Reset**, which takes the setting away so the level from higher up counts again. A cell you did not change says where its level comes from, for example *From Acme Corporation*.

Example: to hide Cost center from Contributors in Acme Corporation, open the field permissions of that project and set Cost center to *Hidden* in the column of the role Contributor. Website, inside Acme Corporation, shows Hidden *From Acme Corporation*. If Website should show it again, set Can see there.

## Limits

- You cannot give a role a level above your own in that place.
- A role's own level cannot be reset, only changed.
- Changes show at once and the page says *Saved* when the server has them. If the server refuses, the old level comes back with a sentence that says why.
