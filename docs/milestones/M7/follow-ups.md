# M7 follow-ups

Points found while building M7 that were accepted for later. Each names where it will be done. The IDs (`M7-F01` …) are what the review refers to. WP-01 uses F01 and up, WP-02 F10 and up, WP-03 F20 and up, WP-04 F30 and up, WP-05 F40 and up, WP-06 F50 and up, WP-07 F60 and up, WP-08 F70 and up, WP-09 F80 and up, WP-10 F90 and up, WP-11 F100 and up, WP-12 F110 and up, WP-13 F120 and up, the exit visual pass and the exit waves F130 and up.

## Carried over from earlier milestones

These rows of earlier milestones name M7, or a package that M7 is, as their target. They keep their old ID; WP-01 marks them "moved to M7" in their old tables, pointing here. When a row is done, it is marked done here and in its old table.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M2-F61 | ARCH-API-OUT is a text check: `answer(…)` still takes any value, and the steps of signing in and the caller's own sign-in data answer through it, named one by one in `tools/checks/api-out-exemptions.toml`. Still open: an alias of a wrapper (`use … as`) or a `Body` made by hand and sent through a function of another name is not found | Its target was "when a second kind of entry point (MCP, M7) answers about the caller" | WP-05: MCP answers through the same safe types; the check reads the answers of `bb-mcp` too, or the row says why not |
| M2-F64 | ARCH-AUTHZ-ONLY finds a judgment of the actor that skips the narrowing of an access key only where the call names `actor.person…` or gives `None` for the scope; `let id = actor.person.id; ctx.evaluate(id, …)` passes | Its target was "when a use case needs the person's ID apart from the actor": an agent's person is that case | WP-04: a type for the actor's evaluation, so the alias cannot judge an agent as its person |
| M2-F65 | The record of the matrix names the entry point of each line (`api` today, done in M3 WP-03); the columns of the assistant and of AI apps are not there | Its target names "M7 (assistant and AI apps)" | WP-04 (the actor kinds), WP-05 (`mcp`), WP-06 (`assistant`) |
| M2-F93 | Every project label in an answer uses the schema `KeyProjectRef` of the access keys | Its target is "when a package touches `access_keys.rs`" | WP-04 (agent keys): rename it to `ProjectLabelBody` |
| M3-F121 | An access key cannot start a change to more than 1,000 issues (`auth.access_key_not_allowed`): the background job runs as a person, and a key has no person to judge again with | Its target waits for "keys that act on their own, with limits a job can judge again", which agents are | WP-04 decides it for agent keys, WP-07 for runs that judge each change again; the row is closed with the decision, or moved on with a reason |
| M4-F133 | The module map has edges that no crate has (`sample --> policy`, `server --> ai` and others) | Its target is "the package that next adds a crate"; M7 gives `bb-ai` and `bb-mcp` their first code and moves the registry (README, placement) | WP-01 writes the map; WP-02 makes it match the crates |
| M4-F190 | The start of an access key, `bb_`, is written by hand in the browser (`KeyRow.tsx`) and again in Rust (`KEY_START`, `crates/bb-auth/src/access_key.rs`) | Its target is "the next package that changes access keys" | WP-04 (agent keys) |
| M4-F195 | On the evening before summer time begins (a 23-hour day) the earliest day of an access key is the day 24 hours from now, so the next calendar day is not offered | Its target is "the next package that changes access keys" | WP-04 (agent keys) |
| M5-F110 | ARCH-DEPS reads a "tests only" edge of the module map as an ordinary one | M7 changes the map | WP-02, with the edges of `bb-ai` and `bb-mcp` |
| M6-F35 | Small leftovers of the busy write: no test makes an issue in the middle of a step busy, and `reason_of` in `many_changes/judge.rs` names a busy database twice | Its target is "the next package that changes changes to many issues" | WP-07 (runs and *Undo the run*) |

## Found in M7

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M7-F01 | The UX page (`docs/02-ux-design.html`) uses utility classes (`u-margin-top-10px` and others) that only `docs/design/mockup.css` defines; the page does not load that file, so they do nothing, and no check finds a class that no loaded style sheet defines | Found while drawing the screens of M7; the old pictures look right without them, and a fix touches the whole page | The first M7 package with screens (WP-05): load the sheet or drop the classes, and a check that every class of the page is defined |
