# Statuses and status changes

A workflow has **statuses** and **status changes**. A status has a name and a status group. Renaming a status changes its name only, so old issues and their history keep working. The first status of a workflow is where a new issue starts.

A **status change** goes from one status to another and has the text of its button, for example *Start working*. The issue page shows the status changes you may carry out now as the choices of the status button, and nothing else: a change that the workflow does not have, or that its rules keep from you, is not offered.

## The starter workflows

A new organization starts with three workflows. They are copies the organization may change.

| Workflow | Statuses (status group) | Used for |
|---|---|---|
| **Simple** | Open (*Not started*), In progress (*In progress*), Done (*Done*); any status can follow any other | Task, Goal, and every issue type that has no workflow of its own |
| **Bug** | Open, In progress, Testing (*In progress*), Done, Won't fix (*Done*) | Bug |
| **Development** | Open, In progress, In review (*In progress*), Testing (*In progress*), Done | Feature |

An organization that used Bugaboo before workflows existed keeps **Simple** for every issue type until somebody changes it. Nobody's issues move by themselves, and every issue keeps its status.

## The status group *Done* and the closed date

An issue that enters a status in the group *Done* gets a closed date; an issue that leaves the group loses it. Moving between two statuses of the group keeps the date it had. In **Bug**, both *Done* and *Won't fix* are in the group *Done*.

## An issue in a status its workflow does not have

This happens when an issue changes its issue type or moves to another project and the workflow there has no such status, or when a workflow that had the status is taken away while the issue is in the trash. The issue shows the name that another workflow gives that status.

Such an issue starts again in the **first status** of its workflow, and only in that one. When you change the issue type or move the issue, it is put there for you, and a status change asked for in the same step is refused unless it goes there. When the status exists in the new workflow, the issue keeps it and any status change you ask for in the same step follows all the rules of that workflow. Changing the type or the project never gets around a rule.
