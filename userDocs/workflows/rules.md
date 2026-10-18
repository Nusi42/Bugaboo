# Rules of a status change

A status change can have four rules. Bugaboo checks every status change the same way, wherever it happens: the issue page, the status button, [changing many issues](../issues/change-many.md) and *Try it* all get the same answer.

- **Who can do it.** The roles, the groups, *the person it is assigned to* and *the person who reported it* who may carry out the change. A change that names nobody is for everybody who may change the issue.
- **Only if …** Something that must be true first: a field has a value, or has none. The words are the same as for *Must be filled in*. A change refused for this reason says that something else must be true first, and names the field when you may see it.
- **Must be filled in.** Fields that need a value before the change. The change is offered with those fields, and a change that leaves one empty is refused at that field with *Must be filled in*.
- **Ask for.** Fields shown in a small form during the change. The change is offered with those fields, and the change has to carry them.

Only the description, *Assigned to* and your own fields can be named in a rule: the other fields always have a value.

## A rule never gives a right

The rules can only narrow what *Change issues* or *Change own issues* already allow. A person who may not change the status of an issue cannot carry out a status change that names them in *Who can do it*.

## A field you cannot see is never asked for

A rule that names a field you may not see turns the change down without naming any field, whether the field has a value or not, so the answer tells you nothing about it. The same holds for a field you can see but not change: the change is refused, and the field is named because you may see it.

## Who may change a field

When you can see a field but not change it, the app can tell you which roles may. It names only roles that somebody holds in the project, and nothing about a field you may not see: for that, the answer is the same as for a field that does not exist. If you are not shown the people of the project by role, no role is named. The workflow editor uses the same answer for *Who can do it*.
