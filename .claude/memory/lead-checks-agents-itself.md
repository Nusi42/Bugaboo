---
name: lead-checks-agents-itself
description: The lead checks on running builders and reviewers about every 15 minutes on its own and acts on finished reports
metadata:
  type: feedback
---

While builders or reviewers run, the lead looks at their progress about every 15 minutes (a recurring session cron job) and acts on a finished report right away (merge, ci, STATUS.md) without waiting to be asked.

**Why:** an agent once finished hours before anyone looked. Not once a minute; 15 minutes is the agreed pace.

**How to apply:** right after starting a background agent, schedule the check (off-minute, every 15 minutes) if none is running. A check looks at the work folder (git log/status, `.work/` file times), messages or restarts an agent that is silent for 30+ minutes, and reports only when something changed. Delete the job when nothing runs. See also [[next-steps]].
