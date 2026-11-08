---
name: clean-work-folders
description: "Disk is tight; once a work package is merged and green, clean its work folder's build output (keep CodeGraph DB and Claude memory)"
metadata:
  type: feedback
---

Each `bugaboo.winN/target` grows to 30 to 110 GB, so the disk fills up.

**Why:** with five work folders the build output alone takes several hundred GB.

**How to apply:**

- After a package is merged into `WIN1` and pushed, run `cargo xtask folder clean <n>` from another work folder (the lead's): it moves `WIN<n>` to `origin/main` and deletes everything Git does not track, keeping `.codegraph/`, `.tools/`, `node_modules` and the local settings. It refuses, changing nothing, while the folder has unpushed or uncommitted work. Do it at the end of the package, not mid-review.
- Keep `.codegraph/`, `.claude/memory/`, `.tools/` and `node_modules/` (small, and needed for the next package).
- Clean `win1`'s own `target/` at milestone ends, not between merges (CI there needs the incremental build).
- Any `winN` that is not working and not waiting for anything gets cleaned at once (the maintainer, 2026-11-08: "the winX which are not doing anything and not waiting for anything should really consider cleaning up their folders"). Check after every landing. A folder's `target/` reached 142 GB (win6) and win1's spare `.work/target-lead` 187 GB.
- Old picture sets in `.work/pics*` and `.work/shots*` of a merged package go too; keep only the newest set while a review page still points at it.
- The lead no longer keeps a second build folder in win1: landing cis run in the idle builder folder of the package being landed.
