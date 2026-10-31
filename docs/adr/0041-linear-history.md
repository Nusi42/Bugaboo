# ADR-0041: Branches are rebased, never merged

Status: **Accepted** · 2026-10-31

## Context

The lead combined each builder's branch with the lead's branch `WIN1` by a merge commit (`Merge the search page`, ADR-0038), and `main` moved to `WIN1` by a fast-forward. The history of `main` therefore has a merge commit for every work package, and so will the public `main`. Each package appears twice in it (its own commits and the merge), and `git bisect` walks into the side branches. The public repository's ruleset on `main` can require a straight line ("Require linear history"); a merge commit would then reach GitHub only on the maintainer's bypass. The sister project made the same decision on the same day.

## Decision

- **Branches are combined by rebasing.** The lead makes a copy of the package branch, rebases the copy onto `WIN1` (conflicts are resolved commit by commit during the rebase), and then moves `WIN1` to it with `git merge --ff-only`. No merge commit is made, on any branch that reaches `main`. Enforced by: GIT-LINEAR.
- **`WIN1`, the master's `main` and the public `main` stay a straight line.** The pre-push hook refuses a merge commit among the commits a push adds, on a work folder's push to the master and on the master's push to the public remote. The commits are chosen the same way as for the private-data guard (on a local branch, a tag or `HEAD`, and on no remote-tracking branch); one function chooses them for both. A plain clone, whose `origin` is no marked master, is not checked. Enforced by: GIT-LINEAR.
- **A builder brings in a newer `main` by rebasing** its own branch onto `origin/main` and pushes it again with `git push --force-with-lease origin WINn`. Only the builder's own `WINn` is ever force-pushed; `WIN1`, `main` and every branch on the public remote never are (review: which branch is pushed with force is the person's choice, not something the hook can tell).
- **The merge commits already on `main` stay;** history is not rewritten. The check leaves out every commit under the tip of `main` on the day of this decision, so the master can still publish them. Enforced by: GIT-LINEAR.
- **No merge subject.** A commit message never has the subject of a merge (`Merge …`); this replaces the `Merge <what it brought>` form of ADR-0038. Git's `Revert "..."` stays. Enforced by: GIT-COMMIT-MSG.
- **The public side:** the ruleset of the public repository requires linear history on `main` ("Require linear history"), so GitHub refuses a merge commit as well (review: the setting lives on GitHub, not in the repository).

## Consequences

- The integration step of the lead is a rebase and a fast-forward; "Life of a work package" and "The lead's routine" in [04-milestones](../04-milestones.md) say so.
- A rebased package gets new commit IDs; the builder's folder is reset to `main` for the next package anyway, and a builder whose branch was rebased by the lead does not push the old one again.
- The `WINn` branch of a builder may be force-pushed between review rounds, with `--force-with-lease`, so it can follow `main`.
- ADR-0038 (the `Merge` subject) and ADR-0039 (the checks of the master's push) are amended by this record, and the integration step of ADR-0024 follows it.

## Alternatives considered

- **Merge commits (the old flow):** keep each package together, but each package appears twice, `git bisect` walks into the side branches, and the public ruleset would need a bypass. Rejected.
- **Squash each package into one commit:** a straight line, but the package's separate commits and their messages are lost. Rejected.

## Enforced by

GIT-LINEAR (no merge commit in what a push adds) and GIT-COMMIT-MSG (no merge subject); review for force-pushes and for the ruleset on GitHub.
