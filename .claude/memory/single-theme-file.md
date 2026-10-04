---
name: single-theme-file
description: All UI theming lives in one theming file (like the sister project); components carry no style values or classes of their own
metadata:
  type: feedback
---

All UI theming stays in a single theming file, not spread over the code. Follow the sister project: one theme file holds tokens with mode variants plus `part` rules; components only emit `data-part` / `data-state` / `data-variant`; lint bans `style=` and literal `className`.

**Why:** an earlier Bugaboo build had 55 CSS modules (3,261 lines), untokened weights, line heights and z-indexes, and per-component forced-colors blocks.

**How to apply:**

- The theme file is `design/theme.kdl` (ADR-0026).
- Compile it at build time to a static CSS file, because Bugaboo's CSP is `style-src 'self'` (the sister project injects styles at runtime with 'unsafe-inline').
- Every UI builder and reviewer prompt checks that no styling lives outside the theme file.
- Related: [[ssot-dry-odr-gate]].
