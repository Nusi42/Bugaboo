import { dataOf } from "./answer.ts";
import type { Api } from "./client.ts";
import { draftKeptCodes, heartbeatSeconds, versionsMax } from "./editing.generated.ts";
import { msPerSecond } from "./time.ts";
import type { Schema } from "./readonly.ts";

/**
 * The calls for editing long text (ISS-17, ISS-11, ADR-0019) that no screen should write for itself:
 * the heartbeat that keeps an edit lock and the draft with it, the drafts of a person, and the
 * check of the versions of what is on the screen. The screens that use them come with the issue
 * page; the times and sizes come from the server code (`editing.generated.ts`).
 */

/** The client the calls go through. */
export type Client = Readonly<Api["client"]>;
/** A record by its kind and ID. */
export type RecordRef = Schema<"RecordRefBody">;
/** One thing being edited: a part of a record. */
export type EditWhat = Schema<"EditWhatBody">;
/** What a draft is for: a part of a record, or the form that makes a new record in a project. */
export type DraftTarget = Schema<"DraftTargetBody">;
/** The answer to asking for a lock: it is taken, or somebody else is editing. */
export type LockOutcome = Schema<"LockOutcomeBody">;
/** The answer to a heartbeat. */
export type Heartbeat = Schema<"HeartbeatAnswerBody">;
/** The text so far of a draft, with the version it started from. */
export type DraftText = Schema<"DraftTextBody">;

/** Whether a refusal of a save is one that keeps the text as the person's draft (the list of the server). */
export function keepsTextAsDraft(code: string | undefined): boolean {
  return code !== undefined && (draftKeptCodes as readonly string[]).includes(code);
}

/** The ID of an open editor: made when the editor opens, one for each tab. */
export function newEditorId(): string {
  return crypto.randomUUID();
}

/** The lock on a thing, taken on the first keystroke (`status` says if somebody else is editing). */
export async function takeEditLock(client: Client, what: EditWhat, editorId: string) {
  return dataOf(await client.POST("/api/v1/edit-locks", { body: { what, editor_id: editorId } }));
}

/** Who is editing a thing now, without taking the lock; `null` when nobody is. */
export async function readEditLock(client: Client, what: EditWhat) {
  const state = dataOf(
    await client.GET("/api/v1/edit-locks", {
      params: { query: { kind: what.record.kind, id: what.record.id, part: what.part } },
    }),
  );
  return state.held ?? null;
}

/**
 * One heartbeat: keeps the lock and the text so far as the draft of the person, and brings back
 * what others did since the last one (somebody asked to finish, or took the editing over).
 */
export async function sendHeartbeat(
  client: Client,
  what: EditWhat,
  lockToken: string,
  draft?: DraftText,
): Promise<Heartbeat> {
  return dataOf(
    await client.POST("/api/v1/edit-locks/heartbeat", {
      body: { what, lock_token: lockToken, ...(draft === undefined ? {} : { draft }) },
    }),
  );
}

/** Lets go of a lock: the text was saved, or the editor is closed. */
export async function releaseEditLock(
  client: Client,
  what: EditWhat,
  lockToken: string,
): Promise<void> {
  await client.POST("/api/v1/edit-locks/release", { body: { what, lock_token: lockToken } });
}

/** Asks the person who is editing to save and let go (*Ask Ana to finish*); they are told at their next heartbeat. */
export async function askToFinish(client: Client, what: EditWhat): Promise<void> {
  await client.POST("/api/v1/edit-locks/ask-to-finish", { body: { what } });
}

/** Takes the editing over (*Edit anyway*): the lock is the caller's, and the other person's text stays their draft. */
export async function editAnyway(client: Client, what: EditWhat, editorId: string) {
  return dataOf(
    await client.POST("/api/v1/edit-locks/edit-anyway", {
      body: { what, editor_id: editorId },
    }),
  );
}

/** The draft of the person for a target, or `null` when there is none. */
export async function readDraft(client: Client, target: DraftTarget) {
  const slot = dataOf(await client.GET("/api/v1/drafts", { params: { query: queryOf(target) } }));
  return slot.draft ?? null;
}

/** The target as the query of a read: what is not there is left out. */
function queryOf(target: DraftTarget) {
  return {
    kind: target.kind,
    ...(typeof target.id === "string" ? { id: target.id } : {}),
    ...(typeof target.part === "string" ? { part: target.part } : {}),
    ...(typeof target.project === "string" ? { project: target.project } : {}),
  };
}

/** Keeps a text as the draft of the person. */
export async function saveDraft(client: Client, target: DraftTarget, draft: DraftText) {
  return dataOf(await client.PUT("/api/v1/drafts", { body: { target, draft } }));
}

/** Throws the draft of the person away (*Throw away*). */
export async function throwAwayDraft(client: Client, target: DraftTarget): Promise<void> {
  await client.POST("/api/v1/drafts/throw-away", { body: target });
}

/** The groups of `records`, each as long as one check of versions may be. */
export function inGroupsOfOneCheck(
  records: readonly RecordRef[],
): readonly (readonly RecordRef[])[] {
  const groups: (readonly RecordRef[])[] = [];
  for (let at = 0; at < records.length; at += versionsMax) {
    groups.push(records.slice(at, at + versionsMax));
  }
  return groups;
}

/**
 * The versions of the records on the screen, in the order asked. More records than one check
 * takes are asked for in as many requests as needed, all at once.
 */
export async function versionsOf(client: Client, records: readonly RecordRef[]) {
  const checks: readonly Schema<"VersionsBody">[] = await Promise.all(
    inGroupsOfOneCheck(records).map(async (group) =>
      dataOf(await client.POST("/api/v1/versions/check", { body: { records: [...group] } })),
    ),
  );
  return checks.flatMap((checked) => checked.items);
}

/** The timers a heartbeat runs on; tests replace them. */
export interface Timers<Handle> {
  readonly every: (run: () => void, milliseconds: number) => Handle;
  readonly stop: (timer: Handle) => void;
}

/** The end of a beat is the business of whoever sends it; the timer only waits for it. */
function ignore(): void {
  // Nothing to do: a beat that failed is tried again at the next one.
}

/**
 * Calls `beat` as often as the server wants a heartbeat (every 15 seconds) on the given timers
 * until the function it returns is called. A beat that has not ended when the next is due is not
 * run again on top of it, so a slow network does not pile up requests.
 */
export function startHeartbeatsOn<Handle>(
  beat: () => Promise<unknown>,
  timers: Timers<Handle>,
): () => void {
  let running = false;
  const timer = timers.every(() => {
    if (running) {
      return;
    }
    running = true;
    void beat()
      .catch(ignore)
      .finally(() => {
        running = false;
      });
  }, heartbeatSeconds * msPerSecond);
  return () => {
    timers.stop(timer);
  };
}

/** [`startHeartbeatsOn`] on the timers of the browser. */
export function startHeartbeats(beat: () => Promise<unknown>): () => void {
  return startHeartbeatsOn(beat, {
    every: (run, milliseconds) => setInterval(run, milliseconds),
    stop: (timer) => {
      clearInterval(timer);
    },
  });
}
