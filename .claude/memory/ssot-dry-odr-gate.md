---
name: ssot-dry-odr-gate
description: SSOT, DRY and ODR are a mandatory merge gate in every code review, for existing and new code; every reviewer prompt must check it
metadata:
  type: feedback
---

Every source-code review checks SSOT, ODR and DRY and common bad programming patterns, for the diff and the code it touches. A finding blocks the merge. Existing code is held to the same rule.

**Why:** in M2 the reviews kept finding second copies (three `evaluation_of`, the level walk written three times, page-size constants, names kept in step by tests).

**How to apply:**

- Every reviewer prompt names the review gate in coding-standards §1 and asks for SSOT/ODR/DRY findings as must-fix.
- Every builder prompt repeats it.
- LINT-DRY (jscpd) is active in CI; keep it green.
- See also [[clean-work-folders]].
