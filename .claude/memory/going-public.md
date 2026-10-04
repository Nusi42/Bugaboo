---
name: going-public
description: Bugaboo is prepared for its public GitHub repository before M6 goes on; clean everything first, push once
metadata:
  type: project
---

Decision: Bugaboo goes to its public GitHub repository (private at first) before M6 wave 1 continues. The brief is the maintainer's going-public plan for this project, kept outside the repository next to the work folders (one file per project; same steps as the sister project). Order: clean files, tooling, licence and the whole history first, then push exactly once after the maintainer says "push".

**Why:** a force-push after a rewrite leaves the old commits reachable on GitHub; the old history carries the private e-mail as author.

**How to apply:** no push to GitHub before the explicit "push"; M6 wave 1 stays paused (all work folders at the same commit) until the rewrite is done and the folders are moved onto the new history. See [[no-private-info-in-git]].
