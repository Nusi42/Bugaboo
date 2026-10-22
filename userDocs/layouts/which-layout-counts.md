# Which layout counts

A layout is set for the whole organization or for a project, and for all issue types or one. The nearest place wins: the project, then the projects above it, then the organization. At one place a layout for an issue type wins over a layout for all types.

So a sub-project uses the layout of its parent until it gets its own. Customizing it copies what it uses now; *Reset to default* takes its layout away, and the place goes back to the layout that counts there without a layout of its own: for a layout of one issue type, the layout for all types of the same place; for a top-level project, the layout of the whole organization. The editor says which place a form takes its layout from.

A field that is required stays on the new-issue form for everybody who can fill it in, whatever the layout says, so a form can always be filled in.
