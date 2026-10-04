---
name: ux-quality-gate
description: Screens are rejected for bad alignment or spacing; a consistent UX concept and state colors are required before a milestone closes
metadata:
  type: feedback
---

Review finding from M3 and M5: passing tests and token checks did not make the screens look designed. Wrong alignments and gaps, text touching borders, dialogs that cannot be resized and issues in different states not told apart by color were found only in the screenshots. The M5 list again had faint status tints and bold text everywhere, against UX §7.3.

**Why:** nobody reviewed the composed screens as a whole, and a screen gets no second first impression.

**How to apply:** UX quality ranks with correctness and security. Every package with screens gets its own UX review (a reviewer who looks at every picture against UX §1.1 and §7, not only the code), and visual defects block a merge like a failing test. Before a milestone review page, look at every computer and phone screenshot for alignment, padding, consistent sizes and state colors, and fix what is off first. Every new screen follows the approved UX concept. See [[user-reviews-on-phone]], [[single-theme-file]] and [[typography-and-status-colors]].
