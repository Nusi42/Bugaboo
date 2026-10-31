---
name: access-rights-are-deliberate
description: "don't add protection against what a reader can derive from fields they may read; the administrator set those rights on purpose"
metadata:
  type: feedback
---

When a reader may read some fields, Bugaboo does not hide values that could be worked out from them (example: worked-out *Time left* shown without the right to read *Time spent*, though estimate minus time left gives time spent). Each field follows its own permission.

**Why:** the maintainer (2026-10-31): "whoever allowed this access rights does surely do this on specific purpose" — access rights are the administrator's deliberate choice, not something the software second-guesses.

**How to apply:** don't propose masking derived values; hidden fields and hidden issues still never leak through totals or search (PRIV-13). See [[explicit-deletion]] for the same principle of trusting the organization's judgement.
