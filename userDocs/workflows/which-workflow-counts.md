# Which workflow counts

A workflow is used for the whole organization or for a project, and for all issue types or one. The nearest place wins: the project, then the projects above it, then the organization. At one place a workflow for an issue type wins over a workflow for all types. This is the same rule as for [layouts](../layouts/which-layout-counts.md).

So a sub-project uses the workflow of its parent until it gets its own. A project may change the workflow it inherited: *Customize* creates a copy of the workflow that counts there for that place, and the copy counts for the project and its sub-projects. Taking the copy away makes the place follow its parent again.

A place can also use a workflow that exists already, for example the workflow *Bug* for the issue type *Bug* in one project. Several places may use one workflow, and a change of that workflow counts in all of them. A place can use only a workflow that belongs to that place or to a place above it, so a project never uses the workflow of a project beside it.

## Who may change workflows

The permission **Manage workflows** is needed. Administrators have it everywhere and Project leads have it in their projects. An access key never holds it: workflows stay in the app, where you are signed in.

A workflow that is used in several places is changed by somebody who holds the permission at all of them: for a workflow used in one project, a Project lead of that project; for a workflow used for the whole organization, an administrator.

## Issues keep their statuses

When the workflow that counts for an issue changes (it moves to another project, its issue type changes, or a place uses another workflow), the issue keeps its status if the new workflow has it. If the new workflow has no such status, see [An issue in a status its workflow does not have](statuses-and-changes.md#an-issue-in-a-status-its-workflow-does-not-have).

A place cannot be switched to another workflow, and a workflow cannot be taken away, while an issue would be left in a status that the workflow counting afterwards does not have. Move those issues first.
