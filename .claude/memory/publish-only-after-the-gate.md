---
name: publish-only-after-the-gate
description: "move main and push to GitHub only after WIN1's push to the master succeeded; chain every step with &&"
metadata:
  type: feedback
---

Moving `main` in the master fetches WIN1 straight from the work folder, so it does not depend on WIN1's push to the master having passed its pre-push checks. Chain push to the master, the fast-forward and the push to GitHub with `&&` and stop at the first failure.

**Why:** a merge once reached GitHub with a Clippy error and a stale generated page, because the push to the master failed its checks while the fast-forward and the public push still ran.

**How to apply:** one command per publish, every step joined by `&&`, the logs read before reporting. See [[no-private-info-in-git]].
