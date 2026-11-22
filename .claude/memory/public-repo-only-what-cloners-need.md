---
name: public-repo-only-what-cloners-need
description: "The public repo (memory notes included) holds only what anyone cloning it needs; the maintainer's own setup never goes into ADRs, docs, STATUS or memory"
metadata:
  type: feedback
---

The repository is public, and these memory notes are part of it. Committed files (ADRs, docs, guides, STATUS, memory, commit messages) describe only what anyone who clones the project needs to do. How or where the maintainer develops, syncs, stores keys or sets up their computer is private and stays out, even when it explains a decision. Such facts go into the untracked `CLAUDE.local.md` of the master.

**Why:** the maintainer was angry (2026-11-22) when personal storage and setup details were written into a decision record and STATUS. Extends [[no-private-info-in-git]].
**How to apply:** record a maintainer decision as a general rule of the project and drop every personal detail. Say so explicitly in builder prompts, and check their diffs for words about personal storage, sync services or "this computer" before landing.
