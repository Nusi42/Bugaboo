# Runtime

How the parts work together in the most important situations.

## Reading an issue

```mermaid
sequenceDiagram
  participant B as Browser
  participant A as bb-api
  participant U as bb-app
  participant P as bb-policy
  participant S as bb-store
  B->>A: GET /api/v1/issues/WEB-142?fields=title,status,priority
  A->>A: session → actor (or 404-style error)
  A->>U: get_issue(actor, WEB-142, fields)
  U->>P: may actor view WEB-142?
  P-->>U: yes + field mask
  U->>S: load issue (only requested ∩ visible fields)
  S-->>U: row
  U-->>A: Authorized<IssueView>
  A-->>B: 200 JSON + ETag (hidden fields never serialized)
```

If the actor may not see the issue, the answer is the same "not found" as for an issue that doesn't exist (SEC-MATRIX).

## Searching

```mermaid
sequenceDiagram
  participant B as Browser (Simple or Advanced search)
  participant A as bb-api
  participant Q as bb-searchlang
  participant P as bb-policy
  participant S as bb-store
  B->>A: search text or syntax tree
  A->>P: field catalogue for actor (readable fields only)
  A->>Q: parse + check against catalogue
  Q-->>A: syntax tree, or "There is no field called …"
  A->>P: visibility predicates (projects, restricted issues)
  A->>S: compile tree + predicates → one SQL query
  S-->>A: page of rows + next cursor
  A-->>B: rows (only requested ∩ visible fields)
```

- Sorting, grouping and totals are only allowed on readable fields.
- Every query has a time limit, a page-size cap and a complexity limit.
- The browser runs the **same parser** (WebAssembly) for instant feedback; the server's answer is the one that counts.

## Changing something

```mermaid
sequenceDiagram
  participant B as Browser
  participant A as bb-api
  participant U as bb-app
  participant P as bb-policy
  participant S as bb-store
  participant O as Outbox consumers
  B->>A: POST change status (version in the body)
  A->>U: change_status(actor, cmd)
  U->>P: may actor make this status change? (who can do it, only if …)
  U->>S: BEGIN · check version · apply · "must be filled in" rules
  U->>S: append domain event (+ outbox row) · COMMIT
  U-->>A: Authorized<IssueView>
  A-->>B: 200 (or 409 "Ben changed this a moment ago")
  S-->>O: outbox → history, notifications, webhooks, security log, search index, live updates
```

- **Optimistic locking:** every entity has a version, and every change sends the version it read in its body (`version`, ARCH-VERSIONED). A stale version gives the plain-language conflict message and, with it, who changed the record, when, which fields and the values now (ADR-0019 amendment M3); the browser writes the sentence from them.
- **"Then automatically …"** actions run as part of the same use case, as the same actor.
- **Consumers** filter by the **receiver's** permissions before sending anything (e-mail, webhook, live update).

## The assistant proposes, the person decides

```mermaid
sequenceDiagram
  participant B as Browser
  participant U as bb-app assistant
  participant C as Context builder
  participant L as AI provider
  B->>U: "Summarize WEB-142"
  U->>C: collect context as the actor
  C->>C: drop hidden fields and fields excluded from AI · fence untrusted text
  U->>L: prompt + tool list (read tools run, write tools return proposals)
  L-->>U: answer + proposed change (priority → Critical)
  U-->>B: answer (labelled AI) + proposal card
  B->>U: Apply
  U->>U: normal change_status / update path as the actor, marked "via assistant"
```

Outside AI apps (MCP) use the same tools through `bb-mcp`. Their token is the actor's identity narrowed by its scopes; write tools act directly only if the person granted write scope.

## Editing long text (edit locks, ADR-0019)

```mermaid
sequenceDiagram
  participant A as Ana's browser
  participant S as Server
  participant B as Ben's browser
  A->>S: first keystroke in Description → take lock (POST /edit-locks)
  S-->>A: lock token (60 s lease)
  loop every 15 s while editing
    A->>S: heartbeat (renews lease) + private draft; the answer carries the notes of others
  end
  B->>S: open WEB-142
  S-->>B: "Ana is editing the description" (read-only for Ben)
  A->>S: save (text + lock token + base version)
  S-->>A: saved, new version → "Saved"
  S-->>S: lock released
  Note over A,S: If heartbeats stop (tab closed, network lost), the lease runs out and the lock frees itself.<br/>A save with an expired or taken-over token is refused; Ana's text stays as her draft with "Compare and save".
```

- **Short values** use only the version check (no lock).
- **One mechanism for any record:** a lock and a draft are about one *part* of a record named by its kind and ID (`EntityRef`), so issues, comments and documents use the same routes (`/edit-locks`, `/drafts`). A kind of record says where it sits, which version it has and what a person needs to change it by implementing `EditableRecords`; the project is the one kind `bb-app` knows itself. The lease and the heartbeat are the state of an open editor and write no domain event; *Edit anyway*, *Ask … to finish* and an administrator's release are facts others can learn from, so they are commands with an event, and the release is also in the security log. Expired lock rows are deleted when the next lock is taken; a draft is kept 30 days after its last change and deleted by the job `editing.purge-drafts` (and ignored when older). Enforced by: TEST-EDIT-LOCKS.
- **Notes at the next heartbeat:** *Ask … to finish* and *Edit anyway* leave a note for the person who was editing; the heartbeat hands it over once. The heartbeat keeps the text so far as the person's draft also when the lock is gone, and a refused save keeps its text as the draft: nothing typed is dropped. Enforced by: TEST-EDIT-LOCKS.
- **Saving long text** goes through `Ctx::save_text`: inside the change that writes the record it checks the lock token, runs the write (which checks the version) and lets go of the lock and the draft; a refusal is `edit.lock_lost`, `edit.taken_over` or the structured `edit.version_changed`. Enforced by: ARCH-VERSIONED, TEST-EDIT-LOCKS.
- **Confirmed saves:** the browser shows changes at once as *Saving…*, then *Saved* when the server confirms, or a plain explanation (conflict, lock lost, offline) that keeps the person's text.
- **Checking versions:** every 30 s and on focus, the browser asks (`POST /versions/check`, up to 50 records) for the current versions of what's on screen; a newer version by someone else shows *"Ben changed this after you"*.

## Working hours

1. *Start work day* creates a work-day record for today (person, start time, time zone).
2. Breaks and the end are added the same way, or typed in later.
3. The **working-time rules** of the organization's preset are checked after every change. Broken rules become warnings stored with the day; nothing is blocked.
4. Changing a day in the past requires a reason. The event log keeps the old value; the person is notified if someone else changed it.
5. A daily job deletes work days older than the keep-and-delete rule allows.

## Removing a person (GDPR)

1. An admin chooses *Remove a person*. `bb-app::privacy` builds a **plan** from the personal-data marks: what will be deleted now, what will be anonymized, what must be kept by law and until when (DATA-MARKS; legal keep periods: review (compliance.md lists them)).
2. The admin confirms. A background job runs the plan in steps; each step is idempotent and logged.
3. The person's name becomes *Former user K7Q2* everywhere (a tombstone record replaces the user). Records under a legal hold are deleted by the normal deletion schedule.

## Live updates (built last)

The outbox feeds a small in-memory hub. Each open browser tab holds one Server-Sent Events connection and subscribes to what is on screen. Every event is checked against the subscriber's permissions, then sent as a small "this changed" note; the browser refetches only what it shows.
