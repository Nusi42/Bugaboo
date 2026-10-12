# Who sees a field

Each role has one of three levels for every field, own fields too: **Hidden**, **Can see** or **Can change**. Hidden means the field does not exist for that role: it is in no issue, no list, no history and no answer of the programming interface, and asking for it is refused exactly like asking for a field that does not exist.

*Organization settings › Field permissions* has the table. A new own field starts with **Can see** for every role that sees projects, and **Can change** for the roles that can change issues (Team member and up). Whoever makes the field can change that at once.

## For one issue type

Under **More options**, choose an issue type to see and set the levels for issues of that type only. A level set for a type wins over the level for all types at the same place; where nothing is set for the type, the level for all types counts. As everywhere, a project's own setting wins over the one of the organization, and sub-projects follow the project above until they set their own. **Reset** takes a level set for the type away.

## What follows from a level

- **Hidden**: the values of the field are not shown in the issue, in conflicts between two people's changes, or in the history.
- **Can see**: the value is shown. A field filled in by others can be read but not changed.
- **Can change**: the person can fill the field in, where they can change the issue or the project at all.

An access key never has more than its owner, and a key that may only look can see but not change.
