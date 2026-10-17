# Changing a workflow

A workflow is changed as a whole, under the **edit lock** that the person holds (the same lock as for descriptions and layouts), on the version they looked at. Until **Publish**, the changes are a draft that only the editor sees; two people cannot change one workflow at once, and a person who is told that somebody changed it first can look again.

## Publish

*Publish* replaces the statuses, the status changes and their rules by what was sent, and counts the version up. A workflow must have at least one status, names must differ, every status change must go between two statuses of the workflow, and a status change cannot go to the status it starts from.

A new status gets a key taken from its name. A status change names a new status by its name.

## A status that goes

When a published workflow drops a status that issues are still in, *Publish* asks where those issues go. Nothing is published until every such status has a place to go to. After the workflow is saved, the issues are moved by a change to many issues, as the person who published, and each issue has the change in its history. An issue that this person cannot change stays in its old status and can be put right as described in [Statuses and status changes](statuses-and-changes.md#an-issue-in-a-status-its-workflow-does-not-have).

Taking a workflow away is refused while an issue would be left in a status that the workflow counting afterwards does not have.

## Try it

*Try it* picks an issue and a person and tells which status changes the person would get, and why not the others: the answer of the judge for each status change from the status the issue is in. It needs the permission **Manage workflows** in the project of the issue. A field is named only when you may see it too.

## Where you change workflows

The routes below `/api/v1/workflows` (and `/api/v1/issues/{issue}/status-changes` for *Try it*) do all of this; the screens of the workflow editor come in a later step of the product.
