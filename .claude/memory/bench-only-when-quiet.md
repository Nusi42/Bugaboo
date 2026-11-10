---
name: bench-only-when-quiet
description: "Speed measurements (cargo xtask bench, speed specs) run only when no other folder compiles or tests; builders code first and wait for the lead's go to measure"
metadata:
  type: feedback
---

A builder must not measure speed while other work folders compile or run tests on the same machine; the numbers then measure the load, not the code.

**Why:** the maintainer (2026-11-09): "win2 is making speed tests while we are under load with all the other agents compiling?" Earlier bench runs swung from 100 to 420 ms between identical runs for exactly this reason.

**How to apply:** in every brief that involves `cargo xtask bench` or speed specs, say: write the code and the correctness tests first, commit, then wait for the lead's "quiet: bench now". The lead gives it only when no other builder is building or testing (check running agents first), and pauses new builders during the measurement. The landing ci's speed step is judged the same way: a speed failure while other folders run is rerun when quiet. Related: [[lead-checks-agents-itself]].
