# Changing a workflow

You find the editor under **Organization settings › Workflows**, and in the settings of a project under **Workflows**. Choose the issue type the workflow is for, and **Customize** a place that has no workflow of its own. The editor shows the statuses as a plain list, each with the statuses it can go to and the words of the buttons. On a computer you can also choose **Drawing**: the statuses as boxes in a column for each status group, the status changes as arrows, and each way out of a box is a button that opens its rules. A phone shows the list only. Everything works with the keyboard.

Under **Try it** you pick an issue and a person and see which status changes they would get, and why not the others.

A workflow is changed as a whole, by one person at a time, as for descriptions and layouts: while you edit, others see *… is editing*. You change the version you looked at. Until **Publish**, the changes are a draft that only the editor sees; two people cannot change one workflow at once, and a person who is told that somebody changed it first can look again.

## Publish

*Publish* makes your changes count for everybody. A workflow must have at least one status, names must differ, every status change must go between two statuses of the workflow, and a status change cannot go to the status it starts from.

You can name in a rule only the fields you may see in that place and the groups you know. Rules that are in the workflow already stay as they are.

## A status that goes

When a workflow drops a status that issues are still in, *Publish* asks where those issues go, and you choose for each removed status which status of the workflow they go to. The question names the issues you can see and only counts the ones you cannot see, and it says how many you could not move; with those the workflow cannot be published yet. No rule of a status change applies to a move, because the issue has no status change into the new workflow. Every move is checked first with your rights: you must see the issue, may change it and may change its status. If one of them could not be carried out (you may not change that issue, or it changed since), nothing is published. A publish that would move more issues than **Change many issues** takes at once is refused, and the question says how many that is. Otherwise the issues are moved as part of the same step, and each issue has the change in its history. Issues in the trash keep their status; when one comes back and its workflow has no such status, it is put in the first status, and that is in its history.

Taking a workflow away, or switching a place to another workflow, is refused while an issue would be left in a status that the workflow counting afterwards does not have.

## Try it

*Try it* picks an issue and a person and tells which status changes the person would get, and why not the others: the answer for each status change from the status the issue is in. It needs the permission **Manage workflows** in the project of the issue. A change whose rules name a field you may not see is answered with *This status change needs a field that you cannot fill in…*, and nothing about the field.
