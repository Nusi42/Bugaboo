# Fields

A **field** is one piece of information on an issue or on a project: the title, the status, the priority. Bugaboo has some fields from the start. Your organization can add **own fields**: *Severity*, *Platforms*, *Customer*, or anything else it needs to track.

- [Kinds of information](kinds.md): what an own field can hold, from a short text to a person
- [Choices, colors and looks](choices-and-looks.md): *Pick one* and *Pick several* fields, how values are shown, and how to combine or retire a choice
- [Rules](rules.md): required, ranges, lengths and patterns, and what you are told when a value is not allowed
- [Who sees a field](who-sees-a-field.md): the levels of each role, also for one issue type

## Creating a field

*Organization settings › Fields* lists the fields. It is for people who hold the permission "Manage fields", which administrators have at the start. Below the list, **Create a field** starts with the question *What kind of information?*. Choose a kind, say whether the field is for issues or for projects, give it a name, and, for a field with choices, type one choice on each line. The page of the new field has the rest.

A field page has these sections: **Basics** (name, help text, whether it has to be filled in, the value a new issue starts with, and the mark that keeps the field from the assistant), **Choices**, **Rules**, **Look**, **Where it is used**, **Who can see it** and **Retire the field**.

## Changing and retiring

If two people change a field at the same time, the second one is told that someone changed it first and can look again.

A field is never deleted. **Retire** stops new values and hides the field from the screens where values are filled in, while the values already set stay on the issues and the projects. You can offer the field again at any time.

## Where a field is used

A field is used in the whole organization, or only in some projects (with their sub-projects), and for all issue types or only some. *Where it is used* lists the projects and issue types that use the field before you change anything. It lists only the projects you can see.

## Own fields on projects

A field created for **projects** is shown on the overview of each project it is used in, and people who may change the project's settings fill it in there.
