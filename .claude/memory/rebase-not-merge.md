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

**How to apply:** follow "The lead's routine" (step 7) and "Working in a folder" in `docs/04-milestones.md`; the
reasons and limits are in ADR-0041. GIT-LINEAR and GIT-COMMIT-MSG enforce it.

Related: [[publish-only-after-the-gate]], [[next-steps]]
