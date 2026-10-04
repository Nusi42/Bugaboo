---
name: clean-work-folders
description: Disk is tight; once a work package is merged and green, clean its work folder's build output (keep CodeGraph DB and Claude memory)
metadata:
  type: feedback
---

Each `bugaboo.winN/target` grows to 30 to 110 GB, so the disk fills up.

**Why:** with five work folders the build output alone takes several hundred GB.

**How to apply:**

- After a package is merged into `WIN1` and `cargo xtask ci` is green, run `cargo clean` in that builder's folder (and remove `test-results/` and `target/sample*` leftovers). Do it at the end of the package, not mid-review.
- Keep `.codegraph/`, `.claude/memory/`, `.tools/` and `node_modules/` (small, and needed for the next package).
- Clean `win1`'s own `target/` at milestone ends, not between merges (CI there needs the incremental build).
