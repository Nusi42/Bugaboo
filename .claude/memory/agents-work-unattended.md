---
name: agents-work-unattended
description: "Bugaboo must be able to run agents with no person in the loop: standing rules hand over what they find; an admin may allow rules that never run out"
metadata:
  type: project
---

The maintainer (2026-11-09): "if bugaboo can't be setup such that it doesn't require human interaction, it's bad and we should change it." An agent still opens attempts only on issues handed to it.

Decisions (2026-11-09, answered in the lead's question):

- A standing rule can hand over the issues its search finds; the agent takes them and works in attempts with runs, so *Throw away* and *Undo the run* stay complete. The old rule option "Change only these fields" (changes without an attempt) is dropped; on rule-only issues an agent only reads and proposes (M7-F120 fix).
- Rules keep 30 days by default and 90 as the normal maximum, but a new organization setting lets an administrator allow rules that never run out (still stoppable, limits still apply).
- With *Accept by itself when its checks pass* on, the whole loop runs without a person.

**Why:** unattended operation is a product requirement, not an option to leave out.

**How to apply:** built as M7 WP-14 (after WP-13). Check new agent features against "can this run with nobody watching?". Related: [[next-steps]].
