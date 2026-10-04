# ADR-0019: Edit locks and confirmed saves

Status: **Accepted** · 2026-10-04

## Context
Bugaboo has no real-time co-editing in 1.0. Without care, two people editing a long description, a comment or a document page overwrite each other, and a save that looks done can silently fail. The UI should feel instant (optimistic), but nobody may lose text.

## Decision
**Two mechanisms, one for each kind of value:**

1. **Short values** (status, priority, people, dates, choices): no lock. The change shows at once and is sent with the entity's **version** (`If-Match`). A stale version gets a conflict answer.
2. **Long text** (descriptions, long-text own fields, a comment being edited, document pages, and admin editors such as workflows, layouts and themes): an **edit lock**.
   - **Taken on the first keystroke**, not on opening, so readers never block anyone.
   - **A lease, not a permanent lock:** stored in the database (`edit_lock`: what, who, session, token, expires at). It lasts 60 s and is renewed by a heartbeat every 15 s while the editor is open. Closing the tab, losing the network or a crash simply lets it run out.
   - **Others see** *"Ana is editing the description"* and the field is read-only for them. They can *Ask Ana to finish* (a notification) or, with permission, *Edit anyway*: the lock moves to them, Ana is told, and Ana's unsaved text is kept as her draft.
   - **Drafts** are saved privately on the server every few seconds and in the browser (IndexedDB), so a crash, reload or lost lock never loses text.
   - **Saving** sends the text with the lock token and the base version. The server rejects a save without a valid token (lock lost or taken over) or with a stale version.
   - Admins can release a stuck lock; it's recorded in the security log.

**Confirmed saves (both kinds):**
- The screen shows the change immediately and marks it *Saving…*.
- The server's answer **confirms** it (*Saved*), or the UI explains exactly what happened and keeps the person's text:
  - *conflict* — "Ben changed this a moment ago. [Compare]";
  - *lock lost or taken over* — "Your editing time ran out / Ben took over. Your text is kept. [Compare and save]";
  - *offline* — "Not saved yet. We'll try again when you're back online." Pending saves survive a reload.
- **Watching open items:** until live updates exist (ISS-14), the browser checks the versions of what's on screen every 30 s and on focus (one small request). If someone changed something you just saved, you see *"Ben changed the description after you. [Show changes]"*.

## Consequences
- Nobody loses text silently; every failure has a plain explanation and a way forward.
- Locks need no live-update channel; plain HTTP heartbeats work everywhere.
- Lock records hold person IDs; they are deleted when they expire (no keep period).

## Alternatives considered
- **Real-time co-editing (CRDT, Yjs):** much more complex; planned for later (DOC-10).
- **Locks taken when opening:** block people who only read. Rejected.
- **In-memory locks:** lost on restart and invisible to admins. Rejected; the database is cheap enough.

## Enforced by
ARCH-VERSIONED, TEST-EDIT-LOCKS (see `tools/checks/catalog.toml`).
