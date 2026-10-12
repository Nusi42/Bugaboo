# ADR-0019: Edit locks and confirmed saves

Status: **Accepted** · 2026-10-04

## Context

Bugaboo has no real-time co-editing in 1.0. Without care, two people editing a long description, a comment or a document page overwrite each other, and a save that looks done can silently fail. The UI should feel instant (optimistic), but nobody may lose text.

## Decision

**Two mechanisms, one for each kind of value:**

1. **Short values** (status, priority, people, dates, choices): no lock. The change shows at once and is sent with the entity's **version** (in the body, see the amendment). A stale version gets a conflict answer.
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

## Amendment (M3)

- **The version travels in the body.** Every change sends the version it read as `version` in the request body, as the M2 routes already do, not in `If-Match`; a change of many issues carries one version per issue, which a header cannot. ARCH-VERSIONED reads the body of every change operation in the API description; a route without a version (signing in, making a new record) is named with its reason in an exemption file.
- **The conflict answer is structured.** `edit.version_changed` (409) holds who changed it (a `PersonRef` for the reader), when, which fields, and the current values the reader may see; the browser writes the sentence from it with the reader's field mask and *Who sees people*, so an error message keeps no placeholder (architecture §7, *Errors*). For short values the sentence is "Ben changed Priority to Critical 2 minutes ago." with *Keep Ben's* and *Use mine* (decision M3-Q5); for long text it stays *Compare and save*.
- **Drafts** are saved with the heartbeat (every 15 s) and kept until the text is saved or thrown away, at most 30 days after their last change (decision M3-Q9); a draft for a new issue belongs to the person, the project and the form. A job deletes the old ones (ADR-0017).
- **Ask … to finish and Edit anyway** reach the person who is editing at their next heartbeat, in the open page; e-mail and in-app notifications add the same in M4 (decision M3-Q3).

## Amendment (M3 WP-04)

- **Numbers.** The lease (60 s), the heartbeat (15 s), the keep period of a draft (30 days) and the most records one check of versions takes (50) are named constants in `bb-core::edit`; the browser client reads them from the generated `editing.generated.ts`.
- **Any record.** A lock and a draft are about one *part* of a record named by its `EntityRef`. A kind of record says where it sits, which version it has and what a person needs to change it (`EditableRecords`); the editing code names no kind. Routes whose permission depends on the kind declare `by-record`; *Edit anyway* and the release of a stuck lock declare the permission `edit-anyway`; *Edit anyway* also needs the right to change the record (a lock that could not be kept is not taken), the release of a stuck lock does not (an administrator ends it, whatever the record).
- **Events.** The lease, the heartbeat and the draft write no domain event (they are the state of an open editor); *Edit anyway*, *Ask … to finish* and an administrator's release write one, and the release also writes `edit_lock.released` into the security log, but only when it ended the lease of another person that was still running; that person finds a note at their next heartbeat saying who released it.
- **What a lock stores.** The person who holds it and the editor (tab) ID, not a session. The token is only half the proof: the heartbeat, the release and the save also compare the holder with the person who asks, so a token that leaks to another signed-in person opens nothing. Nobody renews a lock after its holder signs out, so the lease ends within a minute (60 s) after.
- **An event that counts the version up holds the change `version`.** The conflict answer is made from the newest event about the record that holds it, never from the newest event of any kind (ask-to-finish, release and group member events do not count). A person's value in such an event is never stored as a raw ID where the conflict can return it. Each kind of record says which of its changes a conflict may show and which hold a person (`EditableRecords::shows`); a name nobody listed stays hidden. The sign-up policy, which sets two settings together, is one record with a version of its own, kept in its own table (`policy_version`) where no change or reset of a setting can touch it; it has one route, and the generic settings routes refuse and do not list its two settings (`changed_only_on_its_own_screen`). The conflict finds the newest event that counted the version up through all the events of the record, a page at a time, so no window cuts it short. The names of the members of an event are defined once (`bb-app` `member::`), for the writer and for the list of what a conflict may show; the stored words did not change. A version counts up in one place (`bb_core::version::next`).
- **Housekeeping.** Until the job runner exists, `bb-server` runs `purge_drafts` (drafts, old notes, lock rows that ran out) when it starts and once a day (M3-F30).
- **Locks and drafts belong to a tab of a signed-in person.** Every lock and draft route needs the session of a browser (`ctx.session_actor`), so an access key is turned away from them, whatever it may change elsewhere.
- **Nothing typed is dropped.** The heartbeat keeps the text so far as the person's draft even when the lock is gone; a save that is refused (`edit.lock_lost`, `edit.taken_over`, `edit.version_changed`) keeps its text as the draft. The notes for the person who was editing are handed over once at their next heartbeat.
- **Enforced by:** ARCH-VERSIONED (every change takes a version, or is named in `tools/checks/version-exemptions.toml`; a save of long text also takes `lock_token`; both are required in the body), TEST-EDIT-LOCKS (server part, both databases; WP-10 adds the browser scenarios).

## Amendment (M3 WP-05)

- **A long-text own field is saved only under the lock.** `PUT /api/v1/issues/{issue}/fields/{field}/text` is the one route that changes it (`saveIssueFieldText`, in `LONG_TEXT_ROUTES`); `PATCH` of an issue and of a project refuse such a field. A refused save, also one refused before it is tried (a rule, a stale version), keeps the text as the person's draft.
- **A new issue may be made with a long text, like the description.** `POST /issues` takes long-text own fields, because nothing is edited yet and no lock can exist; every later change goes through the lock.
- **A long-text field can only be made for issues.** Projects have no lock route for it yet (M3-F49); the server refuses the pair when the field is made.

## Amendment (M3 WP-06)

- **The issue is known to the editing code.** `bb-app` knows the issue like it knows projects, roles and groups, so locks, drafts and the check of versions work on it without `EditableRecords`: `issues/editing.rs` gives the place, the version and the permission to change (*Change issues*, or *Change own issues* for the person who reported it), and `editing/conflict.rs` says which of its changes a conflict may show. `EditableRecords` stays for kinds the program adds from outside (the notes of the tests). A new kind that `bb-app` itself owns is added the same way, in these two places.
- **The description is saved by its own route.** `PUT /issues/{issue}/description` takes `text`, `version` and `lock_token` and saves through `Ctx::save_text` (ARCH-EVENTS accepts that call as the other way a command writes its event). It is listed in `LONG_TEXT_ROUTES`, so ARCH-VERSIONED holds it to the token.
- **The server renders what the page shows.** The answer of `getIssue` holds `description_html`, made by `bb-markdown` with the image hosts of the organization; the OpenAPI description marks such a string with `x-safe-html` (written once in `bb-api-words`), the generated types say `SafeHtml` for it and the client makes the value (SEC-RICHTEXT). The editor's preview reads the hosts from `GET /content/image-hosts`; the content security policy of a document lists them in `img-src` (read at most every 10 seconds).
- **The browser side is a controller.** `web/app/src/shared/editing/long-text.ts` holds the lock on the first key, the heartbeat, the two drafts (`browser-drafts.ts`: one storage key, by person and by what the draft is for, 30 days) and the three save states; `useLongText` puts it on a screen, `useVersionWatch` does the check every 30 seconds and on return to the tab.

## Amendment (M3 WP-06 fix round)

- **A change never carries long text.** `PATCH /issues/{issue}` takes no `description`; a body that names it is refused. ARCH-VERSIONED refuses any change of a record (a body with `version`) that holds a long-text property (`LONG_TEXT_PROPERTIES` in `bb-api-words`) unless the route is marked `x-long-text`. Making a new record has no version and no lock, so it may take the text.
- **Who is editing is asked when the box opens.** `GET /edit-locks?kind&id&part` says who holds the lock, without taking it (the same rules as taking it); the box shows *Ana is editing* before the first key and opens when the holder lets go.
- **Drafts in the browser belong to a person who is known and are gone after *Sign out*.** None is read or written before the page knows who is signed in; a sign-out on purpose forgets them all, a session that is lost keeps them.
