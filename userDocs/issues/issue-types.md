# Issue types

An issue type says what kind of issue it is. Bugaboo starts with four: **Bug**, **Task**, **Feature** and **Goal**. A project can offer only the types it uses, and everybody who creates an issue picks one of them.

## Changing the types

*Organization settings › Issue types* lists the types, with a search box. It is for people who hold the permission "Manage issue types, kinds of relation and layouts", which administrators have at the start. Open a type to see its panel next to the list:

- **Name and icon.** Pick the icon from a fixed list; each icon has a name, so nothing depends on a picture alone. Names must differ, whatever the capital letters.
- **Retire.** A retired type is not offered for new issues, and an issue cannot be moved to it. The issues that have it keep it and can still be changed. You can offer the type again at any time with the same switch. A type is never deleted.

Below the list, "Create an issue type" adds a type at the end of the list. It is a type of the whole organization.

If two people change a type at the same time, the second one is told that someone changed it first and can look again.

An access key can never create or change issue types: they stay in the app, where you are signed in.

## Offered in

A type is offered in every project until a line says otherwise. A line names a project (it counts for the projects inside it too) or the whole organization, and says **Offered** or **Not offered**. The nearest line wins: a project inside another follows the project above it until it has a line of its own.

- On the panel of a type in *Organization settings › Issue types*, **Offered in** lists the lines, with **Take away** next to each, and **Add a line** adds one. To keep *Bug* to the project *Website*, add a line for the whole organization (*Not offered*) and a line for *Website* (*Offered*).
- In the settings of a project, the tab **Issue types** has a switch for each type. A **Project lead** switches off the types their project does not use. The switch is for that project and the projects inside it, and nowhere else. **Follow the project above** takes the project's own line away.

A project that does not offer a type does not show it in **New issue**, and the server refuses a new issue of that type, or a change of an issue to that type. Issues that have the type keep it. When you move an issue to a project that does not offer its type, the issue moves and keeps the type; the preview says "Bug is not used in Lab. The issue keeps it." A retired type is the same: it is never counted as missing.

## Types created in a project

On the tab **Issue types** of a project, "Create an issue type for this project" adds a type that is offered in that project and the projects inside it. A **Project lead** looks after the types created in their project: they can rename and retire them, and switch them off in a project inside theirs. The types of the whole organization stay with the people who hold the permission for the whole organization. The permission is judged where the type was created.
