---
name: readme-with-every-push
description: "keep README.md (front page, roadmap picture, version) current in the same landing that changes what is built; a milestone's tag goes on the commit that holds its README and version"
metadata:
  type: feedback
---

Update README.md together with every landing that changes what Bugaboo can do, not afterwards: the "Built today / Planned" lines, the tour and feature summary, "What comes next", the roadmap picture `docs/media/roadmap.svg` (done/next tiles, bar, flag), "N of 16 milestones are done", and the version badge. When a milestone closes, the same commit carries the version of ADR-0040 (`Cargo.toml`, Cargo.lock, regenerated `openapi.json`) and the `mN-done` tag points at that commit.

**Why:** the maintainer (2026-11-08), after I tagged m6-done on a commit whose README still said time tracking was "next": "maintaining the readme along the pushes is kind of the whole point." The public front page on GitHub must match what is on main.

**How to apply:** before every GitHub push, read README.md against what the landing adds and fix it in the notes commit. At a milestone close: README + roadmap + version in one commit, then tag it, then push. Related: [[publish-only-after-the-gate]], [[next-steps]].
