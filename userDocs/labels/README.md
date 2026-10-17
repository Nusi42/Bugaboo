# Tags and labels

Tags and labels sort issues. Both show on the issue page, in the **Labels and tags** block of the details, as small colored words.

- A **tag** is a free word. Anybody who may change an issue can add a tag to it, and a tag that does not exist yet is created by adding it. Type a few letters and Bugaboo offers the tags of the project that fit.
- A **label** is a word from a list that the people who look after labels keep. Labels can be in a **label group**, for example *Area* with *invoices* and *billing*. A label group can be a **group to pick one from**: an issue then has at most one label of it.

Whether you see tags and labels on an issue, and whether you can change them, follows your role: they are the fields *Tags* and *Labels*, with the levels you have for them (see [who sees a field](../fields/who-sees-a-field.md)). What you may not see is not shown, and nothing says that it is there.

## Putting tags and labels on an issue

1. Open the issue and, on a small window, press **All details**.
2. Under **Labels and tags**, type a tag and press **Add tag**, or choose a label and press **Add label**.
3. **Take off** next to a tag or a label removes it again.

Adding and taking off are in the history of the issue, and they do not change the version of the issue, so adding a label never makes somebody else's edit a conflict.

A tag can be limited to some projects. In another project it is not offered while you type, and adding it there says so in a sentence.

## Looking after labels and tags

*Organization settings › Labels and tags* is for people who hold the permission "Manage labels and tags". There are three lists:

- **Label groups:** create a label group, rename it, turn it into a group to pick one from, or **retire** it. A label group cannot become a group to pick one from while an issue already has two of its labels: Bugaboo says so, and you take the extra labels off first.
- **Labels:** create a label (name, color, label group), change them, or retire it. A retired label is not offered for new issues, and the issues that have it keep it.
- **Tags:** rename a tag, change its color, and choose the projects it is for, or *Use in all projects*.

Colors are the twelve colors of the theme, so a label is readable in every theme.

### Label groups per project

A label group is created for the whole organization, or in a project. On the tab **Labels** of a project's settings, a **Project lead** creates a label group for the project and the labels in it, and switches groups on and off for the project with a switch each. A group created in a project is offered there and in the projects inside it. A group of the whole organization is offered everywhere until a line says otherwise: on the panel of a group, **Offered in** lists the lines, and **Add a line** adds one, the same way as for issue types (see [issue types](../issues/issue-types.md)). A person looks after a group where it was created: the permission "Manage labels and tags" is judged there.

A project that does not offer a group does not offer its labels in **Add a label**. Labels that an issue has stay on it, also after the issue moves to another project, and they are shown the way a retired label is shown.

### Rename and combine show what changes first

Renaming a tag or label changes it on every issue that has it. Before you save, **Show what the new name changes** lists the issues that change, a sentence on top (*118 issues will change*) and then each issue, ten at a time with **Show all**. You only see the issues you may see.

**Combine with another one** moves every issue from the tag or label to the one you choose, and the first one is gone. **Show what combining changes** lists the issues first, and the combination asks once more. Combining cannot be taken back. A label can be combined only with a label, and a tag with a tag.
