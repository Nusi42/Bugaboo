---
name: dont-wait-for-nightlies
description: "never hold any step (wave, package, builder's bench/browser runs) for the 03:00 nightly run or its result; the nightly yields, the work does not"
metadata:
  type: feedback
---

Go on with the next steps at once; do not wait for the 03:00 nightly run or its result before starting a wave, a package, a check, or a builder's bench/load/browser runs.

**Why:** the maintainer (2026-11-05): "do not wait for nightlies to go on with the next steps. just do it." Again on 2026-11-07, after I held WP-12's bench until 06:00 to keep the nightly quiet and a reboot lost the night: "Why are u waiting for nightly runs. My goal is to finish the project. In particular the AI milestones."

**How to apply:** start builders and reviews right after a merge; never schedule a builder's runs around the nightly. A nightly disturbed by our work is acceptable; its result is looked at in parallel. Only the lead's own full ci in win1 stays off 03:00 (CLAUDE.local.md). Plan the next milestone while the last package of the current one finishes, so builders never sit idle. Related: [[lead-checks-agents-itself]], [[next-steps]].
