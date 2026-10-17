# Changing a workflow

A workflow is changed as a whole, under the **edit lock** that the person holds (the same lock as for descriptions and layouts), on the version they looked at. Until **Publish**, the changes are a draft that only the editor sees; two people cannot change one workflow at once, and a person who is told that somebody changed it first can look again.

## Publish

*Publish* replaces the statuses, the status changes and their rules by what was sent, and counts the version up. A workflow must have at least one status, names must differ, every status change must go between two statuses of the workflow, and a status change cannot go to the status it starts from.

A new status gets its key from its name. A status change names a new status by its name.

You can name in a rule only the fields you may see in that place and the groups you know. Rules that are in the workflow already stay as they are.

## A status that goes

When a workflow drops a status that issues are still in, *Publish* asks where those issues go, and they can go only to the first status of the workflow. Every one of those moves is checked first, as a change by the person who publishes. If one of them could not be carried out (the person may not change that issue, or it changed since), nothing is published. Otherwise the issues are moved as part of the same step, and each issue has the change in its history. Issues in the trash keep their status; when one comes back it can start again in the first status.

Taking a workflow away, or switching a place to another workflow, is refused while an issue would be left in a status that the workflow counting afterwards does not have.

## Try it

*Try it* picks an issue and a person and tells which status changes the person would get, and why not the others: the answer of the judge for each status change from the status the issue is in. It needs the permission **Manage workflows** in the project of the issue. A change whose rules name a field you may not see is answered with *Cannot be filled in*, and nothing about the field.
