# Roles and permissions

A role is a named set of permissions, given to people or groups for the whole organization or in a project. Every organization starts with six: **Administrator**, **Project lead**, **Team member**, **Contributor**, **Viewer** and **Guest**. They are copies: you can rename, change and delete them like your own roles.

*Organization settings › Roles & permissions* lists the roles, in the order of roles. Press a role to open its page.

## Making a role

Under "Make a role", type a name and choose where to start: an empty role, or a copy of the role that is closest to what you need. The new role opens on its page, where you choose what it allows.

On the page of a role, "Copy this role" does the same from there.

## The page of a role

- **Name**: change it and press "Save".
- **What this role allows**: the permissions are grouped by topic (Projects, People, Settings), each a box with a plain sentence. Tick or untick; a change is saved at once, with "Saving…" and "Saved" next to it. A permission that counts only for the whole organization says so: a role given in a project that holds it gives nothing for it.
- **Two-step sign-in for everyone with this role**: everybody who has the role must set up a code app or a passkey. The page says how many people with the role have not done it yet.

Bugaboo asks first when a change would take away **your own** way to manage roles, or to decide who has which role: taking a permission out of a role you have, changing it, or deleting it. It asks only then; the page says which of the two you would lose. Bugaboo never leaves nobody who can manage roles, but it could be nobody but others.

You can only add a permission to a role that you hold yourself.

## Reset to the starting permissions

A starting role has "Reset to the starting permissions": the permissions and the two-step switch it had at the start come back, and the name stays. A role you made yourself has no starting permissions, so it has no reset.

## Deleting a role

A role that nobody has is deleted at once, and cannot come back.

A role that somebody has is deleted only with **the role that takes its place**: pick it in the dialog, and everybody who had the old role gets the other one, in the same place and with the same reach. The dialog says so before anything happens. You can pick a role only if you hold all its permissions in those places yourself, and Bugaboo refuses a deletion that would leave nobody who can manage roles.
