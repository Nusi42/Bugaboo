# Sub-issues and relations

## Sub-issues

An issue can sit under another issue, its **parent**. The issue under it is a **sub-issue**. This is *Part of* on the issue. Use it to cut a large piece of work into smaller ones.

- An issue sits under at most one parent, in any project where you may change both issues.
- An issue never sits under itself or under one of its own sub-issues, however far down. Bugaboo says so in a sentence and changes nothing.
- The parent shows its **progress**: how many of its sub-issues are done. Only the sub-issues you may see are counted, so the numbers are never a hint about issues that are hidden from you.
- Taking an issue out from under its parent is the same step with no parent.

## Relations

A **relation** links two issues in words that you choose from a list, for example *blocks* and *is blocked by*. Each issue shows the relation in its own words: on one issue you read "blocks WEB-12", on the other "is blocked by WEB-7".

- You can add a relation from either issue, and take it away from either issue. An issue is related to another by one kind once.
- An issue you may not see is not shown, and nothing says that it exists.
- Adding and taking away a relation is in the history of both issues.

## Kinds of relation

*Organization settings › Kinds of relation* lists the kinds. Bugaboo starts with four: *blocks / is blocked by*, *duplicate of / has duplicate*, *causes / is caused by* and *related to*. A kind whose two names are the same reads alike from both sides.

People who hold the permission "Manage issue types, kinds of relation and layouts" can create a kind, change its words, and **retire** it. A retired kind is not offered for new relations, and the relations that use it stay. Names must differ, whatever the capital letters, in either direction.

## Date effect and dependencies

A kind of relation has a **date effect**: *None*, or *Must finish first*. A relation of a kind with *Must finish first* is a **dependency**: the issue it starts from must finish before the other one starts. *Blocks* starts with it. Choose the effect on the page of the kind, under *Date effect*.

- At most two kinds can have *Must finish first* at one time. Giving it to a third kind is refused, and the sentence names the two that have it. Take it away from one of them first.
- A dependency never closes a loop. If *A* must finish before *B* and *B* before *C*, then *C must finish before A* is refused, and the sentence names the loop by the issue IDs you may see. An issue in the loop that you may not see is called *an issue you can't see*, and nothing more is said about it.
- Giving the effect to a kind is refused in the same way when the relations the kind already has would close a loop with the others.
- A loop that was there before *blocks* got its effect stays. Only a relation or a kind that would close a new loop is refused.
- Bugaboo never moves a date because of a dependency. The plan shows a warning when the dates break one (the timeline comes later in M8).
