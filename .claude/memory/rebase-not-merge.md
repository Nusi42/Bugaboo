---
name: rebase-not-merge
description: "Maintainer decision (2026-10-31): combine branches by rebasing, never with merge commits; the pre-push hook refuses them (ADR-0041)"
metadata:
  type: feedback
---

Rule: a package branch is rebased onto `WIN1` and `WIN1` is moved forward to it; never a merge commit, on any branch
that reaches `main`. `WIN1`, the master's `main` and the public `main` stay a straight line.

**Why:** the maintainer decided it on 2026-10-31, as in the sister project. Merge commits make each package
appear twice and send `git bisect` into side branches; the public `main` has the ruleset rule "Require linear history".

**How to apply:** in `win1`: `git fetch origin && git switch -C integrate origin/WINn && git rebase WIN1`, resolve
conflicts commit by commit, then `git switch WIN1 && git merge --ff-only integrate`. A builder brings in a newer `main`
with `git rebase origin/main` and pushes its own branch with `git push --force-with-lease origin WINn`; never force-push
`WIN1`, `main` or the public remote. GIT-LINEAR refuses merge commits on pushes to and from the master; GIT-COMMIT-MSG
refuses a `Merge …` subject. Old merge commits on `main` stay.

Related: [[publish-only-after-the-gate]], [[next-steps]]
