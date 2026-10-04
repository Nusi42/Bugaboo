---
name: plain-language-no-vendor-terms
description: Bugaboo must not copy other vendors' naming (Jira etc.); the UI aims at non-technical users, plain language unless a term is truly common
metadata:
  type: feedback
---

Never clone naming schemes from other vendors (for example Jira's epic, transition, post-function, fix version, issue security level, swimlane, JQL-style "BQL"; Confluence "space"). UI and usability target people less technical than the average person: plain language always, unless the term is genuinely common (issue, workflow, dashboard, Gantt, passkey, MCP).

**Why:** a product with its own plain words is easier to learn, and copied vendor terms carry their meaning with them.

**How to apply:** `docs/glossary.md` is the terminology SSOT: check it before naming anything in docs, UI strings or APIs, and add new terms there first. Related: [[user-reviews-on-phone]]
