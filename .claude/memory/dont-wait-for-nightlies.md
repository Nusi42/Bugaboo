---
name: dont-wait-for-nightlies
description: "never hold the next step (new wave, new package, follow-up work) until a nightly run has finished; just go on"
metadata:
  type: feedback
---

Go on with the next steps at once; do not wait for the 03:00 nightly run or its result before starting a wave, a package or a check.

**Why:** the maintainer (2026-11-05): "do not wait for nightlies to go on with the next steps. just do it."

**How to apply:** start builders and reviews right after a merge. Look at nightly results when they arrive, in parallel. Still keep the lead's own full ci from running across 03:00 in win1 (the nightly runs there). Related: [[lead-checks-agents-itself]].
