# Moving a project

Moving puts a project, with everything inside it, into another project, or at the top level. You do it in *Project settings*, with "Move".

Moving needs the permission "Move sub-projects into or out of the project" in the project it leaves **and** in the project it goes into. The top level counts as the whole organization: moving to or from the top level needs the permission for the whole organization.

## Read first, then confirm

Pick where the project goes. Before anything happens, Bugaboo tells you in plain sentences what changes for people:

- who gets access to a project, and with which roles
- who loses access
- who changes role
- what a person can now do, or no longer can do
- which fields and which kinds of people information somebody sees differently
- when a No access starts or stops holding somebody back

Moving a project changes who has a role in it, because it changes which projects are above it. The list holds only projects you can see. People may also be affected in projects you cannot see, and the list says so in one sentence, without a number.

When you press "Move the project", Bugaboo checks that nothing the preview was created from has changed since you read it: the projects, who has which role, or the levels the roles set. If something changed, you are asked to look at the preview again.

A project cannot be moved into itself or into one of its own sub-projects.

## A move that is not allowed

A **No access** on a project keeps a person out of it and out of the projects inside it. People who decide who has which role *above* a project can always take such a No access away again, so nobody is locked out by a project below them.

That is why some moves are not allowed. If moving a project under another one would give somebody that right above a No access that applies to them, they could take away their own No access. Bugaboo does not allow such a move: the preview says "This move is not allowed" and names the person and the project. Take the No access away on purpose first (or give the role differently), or move the project somewhere else.

The same is told in the preview when a move takes a project out from under a No access: the people it held back are named with "a No access no longer holds … back".
