---
name: no-private-info-in-git
description: No private information in Git: no profile paths, machine names, session IDs or links, no personal notes; no Claude-Session trailer
metadata:
  type: feedback
---

Nothing private about the maintainer may end up in Git: commits, commit messages, files or committed memory.

**Why:** published history is permanent, and a private value in it cannot be taken back.

**How to apply:**

- Write paths as `%USERPROFILE%\...` or with the placeholders of the docs, never with the account name; test data uses made-up machine and person names.
- Commits end only with the attribution line of coding-standards §2, even when a session reminder asks for a session link.
- Memory files carry no session IDs and no remarks about a person; only project rules and preferences about the work.
- Links to published pages (`claude.ai/artifact/...`) are private too: keep them outside Git and name the page in docs instead.
- Every builder prompt repeats this. Related: [[ssot-dry-odr-gate]], [[user-reviews-on-phone]].
