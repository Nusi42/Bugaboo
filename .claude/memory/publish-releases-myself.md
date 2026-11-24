---
name: publish-releases-myself
description: "after the maintainer approves a milestone, the lead publishes the GitHub release itself (tag, signed folder, notes)"
metadata:
  type: feedback
---

The maintainer approved the M9 exit with "publish on ur own" (2026-11-24): the lead builds the release from the tag, signs it with the release key, checks it and publishes it as the GitHub release of the tag, without handing the publishing step back.

**Why:** the maintainer does not want to do the publishing by hand; docs/releases/README.md marks signing and publishing as maintainer steps, and this is the maintainer's go-ahead for the lead to do them.

**How to apply:** at a milestone exit with a release, once the maintainer has approved: version commit, green ci, tags, `cargo xtask release`, `release sign` with the release key, `release check`, push main and tags, then `gh release create` with the notes and every file of the folder. Ask again before using a key in any other situation. Related: [[publish-only-after-the-gate]], [[readme-with-every-push]].
