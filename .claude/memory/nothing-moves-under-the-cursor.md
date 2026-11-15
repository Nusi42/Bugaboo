---
name: nothing-moves-under-the-cursor
description: "The maintainer's UX rule: a change moves only what changed; no reload flashes, no jumps under the pointer; checked on every screen"
metadata:
  type: feedback
---

A nice UI is one where nothing moves below the cursor. After a person's action only the changed thing may change size or place; background reads replace content in place, with no grey shapes, no empty lists, no scroll or focus jump. The maintainer (2026-11-13), after tags on an issue page made "everything reload" and creating an issue "flickered": "ALL OF BUGABOO IS BAD AT THIS!!! and it needs a full review pass!"

**Why:** the maintainer found it on the issue page and when creating issues; every earlier picture review missed it, because still pictures cannot show movement.

**How to apply:** M8 WP-13 holds the rule (UX §6) and the measure (layout shifts plus landmark boxes per frame, architecture §11) for every screen and every change made in place. Every builder and picture reviewer prompt that touches the UI names it, and reviews look at frame samples or recordings, not only stills. Related: [[ux-quality-gate]], [[lead-checks-agents-itself]].
