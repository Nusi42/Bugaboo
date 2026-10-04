---
name: restart-ci-after-memory-stop
description: A full ci that Claude Code stopped for low memory may be restarted without asking again
metadata:
  type: feedback
---

When Claude Code stops a background `cargo xtask ci` because the computer ran low on memory, restart it when needed without waiting for permission.

**Why:** waiting for permission each time stalled the merge queue for half an hour.

**How to apply:** check that nothing else heavy runs (builders idle, no other folder testing), then restart the ci; say in the next report that it was restarted and why. See [[lead-checks-agents-itself]].
