---
name: cheapest-capable-model
description: "Give each task to the cheapest model that can do it: Opus plans/reviews, Sonnet builds to a plan, Haiku does chores; a trial through M8 with fix rounds counted"
metadata:
  type: feedback
---

Before an agent takes a task or hands one on, it asks whether the task can be written as numbered steps with files and expected results. If yes, it writes them and gives them to a cheaper model; if not, the thinking is done on the strong model first. Levels: Opus for plans, package docs, ADRs, code/security review, speed and concurrency; Sonnet for building to a plan, fix rounds with file:line, UX fixes, pictures, picture/words reviews; Haiku for chores (re-running failed tests, follow-up rows, notes commits, regenerated files, renames, doc wording, landing runs and their report, folder cleaning). Written in `docs/04-milestones.md`, "The lead's routine", point 12.

**Why:** the maintainer (2026-11-09) wants simpler packages on cheaper models; this session showed Sonnet builders do well on exact briefs but slip on open tasks (partial hand-backs, mangled escapes, tests after code), and each slip costs a review round.

**How to apply:** set `model` on every Agent call by the level; reviewers no longer default to the lead's model. Count fix rounds per package and model through M8 (a trial: "watch this for a while … if quality isn't decreasing, stick to that; if not, reconsider"). At the M8 exit, report the counts and propose keeping or changing the levels. Related: [[lead-checks-agents-itself]].

**Trial log (M8):** WP-02 Opus, 1 fix round (5 small). WP-05 Sonnet stopped halfway (2026-11-12: core, store and contrast done; use cases, routes, events, lock, settings, browser left as "too many shared parts"), finished by an Opus builder. WP-03 Sonnet: 2 fix rounds (13, then 6 points). WP-04 Sonnet: 1 fix round (13 points, 2 privacy leaks). WP-05 Opus finish: 1 fix round (10 points). WP-07 Opus: 2 fix rounds (16, then 4 points). WP-08 Opus: 2 fix rounds (26, then 17 points) and 5 small points. WP-06 Opus: 2 fix rounds (12 privacy and 16 code points, then 8).
