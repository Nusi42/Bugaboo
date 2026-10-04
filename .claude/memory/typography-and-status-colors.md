---
name: typography-and-status-colors
description: Issue rows are clearly tinted by status group and bold is used only where UX §7.3 Type allows; a check enforces it
metadata:
  type: feedback
---

Requirement from the M5 review: issue rows are clearly colored by their status group (Not started included, all four themes, 4.5:1 contrast), and bold text is used only for titles (issue 24, page/dialog 20), section headings (16 bold) and the Critical/High priority word (UX §7.3 "Type").

**Why:** faint tints were easy to miss, and bold everywhere breaks the written type standard.

**How to apply:** every builder brief that touches the UI names UX §7.3 and the status-group tint; reviewers check weights and row tints in the pictures. A check enforces the bold rule (see [[ux-quality-gate]], [[single-theme-file]]).
