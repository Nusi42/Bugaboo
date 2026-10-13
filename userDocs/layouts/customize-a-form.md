# Customizing a form

You need the permission "Manage issue types, kinds of relation and layouts" in the project. Open the editor from the new-issue dialog with **Customize this form**, or from *Project settings › Layouts* (for the whole organization: *Organization settings › Layouts*).

## Making a layout of your own

At first a form has not been customized: it shows the usual fields to every role. Choose the form and the issue type, then press **Customize for** the project. Bugaboo makes a copy of the layout that counts there now, so you can change it without touching the place it came from. **Use the inherited layout again** takes your layout away; it waits while somebody else is editing the layout.

## Sections and tabs

A layout is a list of **sections**. Give a section a title, and put it under a **tab** by typing the tab's name: sections with the same tab are shown together under it, and a form without tabs has none. The title and the issue type are always asked for.

## Moving fields

Each field has a handle. Drag it into a section or onto the list *Fields that are not on this form* to take it off. Everything a drag does has buttons too, so it works with a keyboard, with assistive technology and on a phone, where there are no handles (the handle is not read out by assistive technology): **Move up**, **Move down**, **Move to another section**, **Take off this form**, and **Add to a section** for the fields that are not on the form.

## Roles

Under each field is a list of roles with a box each: the roles the field is shown to (on the new-issue form: asked from). A field you add starts with every role ticked. Choosing roles needs the permission "Decide what roles see" as well: without it you move fields, but you see no role names, a field keeps the roles it has, and a field you add is shown to every role.

## Saving

Nothing changes for anybody until you press **Save layout**. While you have the editor open, nobody else can change that layout: they are told who is editing, and can ask you to finish or, with the right permission, edit anyway.

If the layout changed while you were editing (somebody saved first), Bugaboo shows the layout as it is now and keeps your changes as a draft: **Put my changes back** replaces what you see with your changes, **Throw my changes away** deletes the draft.
