---
name: no-scheduled-nightly
description: "no scheduled nightly run in any project; speed benchmarks run by hand at each milestone exit on a quiet computer; never wait for a run"
metadata:
  type: feedback
---

There is no scheduled nightly run, in Bugaboo or its sister projects. The Windows task "Bugaboo nightly" was deleted on 2026-11-11. The speed benchmarks (`cargo xtask bench`, both databases, on the 1 M datasets) run by hand at each milestone exit, on a quiet computer. The "7 nights green" and "30-day nightly stability" gates are gone from the plan.

**Why:** the maintainer never asked for it; it came from the agent-written design plan (2026-10-04). Asked on 2026-11-11 and chose "Remove it". Before that, they had said twice not to wait for nightlies (2026-11-05, 2026-11-07): "My goal is to finish the project."

**How to apply:** never install a scheduled task (`cargo xtask nightly --install`) or propose one without the maintainer asking. Never hold a step for a run; go on at once. Bench only when the machine is quiet ([[bench-only-when-quiet]]). Related: [[lead-checks-agents-itself]], [[next-steps]].
