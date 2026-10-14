# Which layout counts

A layout is set for the whole organization or for a project, and for all issue types or one. The nearest place wins: the project, then the projects above it, then the organization. At one place a layout for an issue type wins over a layout for all types.

So a sub-project uses the layout of its parent until it gets its own. Customizing it copies what it uses now; taking its layout away makes it follow its parent again. The editor says which place a form takes its layout from.

A field that is required stays on the new-issue form for everybody who can fill it in, whatever the layout says, so a form can always be filled in.
