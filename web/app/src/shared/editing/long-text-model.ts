import type { DraftKeptCode, ErrorCode, Heartbeat, LockOutcome, Schema } from "@bugaboo/api-client";

/** Somebody else is editing: who, since when, and whether the reader may take over. */
export type HeldBySomebody = Schema<"HeldLockBody">;
/** A person as the reader may see them. */
export type Who = Schema<"PersonRef">;

/** Why a text is *Not saved yet* and kept. */
export type LostReason =
  /** The editing time ran out (no heartbeat for a minute) and somebody else has the lock now. */
  | "ran-out"
  /** Somebody used *Edit anyway*. */
  | "taken-over"
  /** The record changed after the person started. */
  | "version-changed";

/** Where the editing stands. */
export type Phase =
  /** Nobody is editing; the text is shown. */
  | { readonly kind: "reading" }
  /** The box is open and nothing was typed: no lock, so just looking blocks nobody. */
  | { readonly kind: "open" }
  /** The first key was pressed and the lock is being asked for. */
  | { readonly kind: "taking" }
  /** The person has the lock. */
  | { readonly kind: "locked" }
  /** The text is on its way to the server. */
  | { readonly kind: "saving" }
  /** Somebody else is editing: the box is read-only. */
  | { readonly kind: "held"; readonly lock: HeldBySomebody }
  /** The text was not saved and is kept; the person decides with *Compare and save*. */
  | { readonly kind: "lost"; readonly reason: LostReason; readonly by?: Who };

/** A draft found when the box opened, offered as *Continue your draft*. */
export interface OfferedDraft {
  readonly text: string;
  /** When it was kept, as a moment in time. */
  readonly savedAt: string;
  /** The version of the record the draft started from; `null` for a new record. */
  readonly baseVersion: number | null;
}

/** Everything a screen shows of an editing in progress. */
export interface Editing {
  readonly phase: Phase;
  /** The text in the box. */
  readonly text: string;
  /** The text as saved now. */
  readonly saved: string;
  /** The text as saved when the box was opened: a change in `saved` since is *Show changes*. */
  readonly startedFrom: string;
  /** The version of the record now. */
  readonly version: number;
  /** A draft that the box can start from. */
  readonly offered: OfferedDraft | null;
  /** Somebody asked the person to finish. */
  readonly asked: Who | null;
  /** The error of the last thing that failed and was not the lock's business, if any. */
  readonly problem: unknown;
}

/** What the controller needs from outside. */
export interface LongTextDeps {
  readonly take: (editorId: string) => Promise<LockOutcome>;
  /** Who is editing now, without taking the lock; `null` when nobody is. */
  readonly peek: () => Promise<HeldBySomebody | null>;
  readonly editAnyway: (editorId: string) => Promise<LockOutcome>;
  readonly askToFinish: () => Promise<void>;
  readonly heartbeat: (
    token: string,
    draft?: { readonly text: string; readonly base_version: number },
  ) => Promise<Heartbeat>;
  readonly release: (token: string) => Promise<void>;
  /** Saves the text under the lock; rejects with the error of the server. */
  readonly save: (input: {
    readonly text: string;
    readonly version: number;
    readonly lockToken: string;
  }) => Promise<void>;
  /** Reads the record again so `setSource` gets the text and the version now. */
  readonly refresh: () => Promise<void>;
  readonly serverDraft: () => Promise<OfferedDraft | null>;
  readonly throwAwayServerDraft: () => Promise<void>;
  readonly browserDraft: {
    readonly read: () => OfferedDraft | null;
    readonly write: (text: string, baseVersion: number) => void;
    readonly forget: () => void;
  };
  /** Starts calling `beat` as often as the server wants; returns the way to stop. */
  readonly startHeartbeats: (beat: () => Promise<unknown>) => () => void;
  readonly newEditorId: () => string;
}

/**
 * Why each refusal that keeps the text as a draft is the lock's business. The codes are the
 * generated list of the server (`draftKeptCodes`), so a code added there fails to compile here
 * until it has a reason.
 */
const lostReasons: Readonly<Record<DraftKeptCode, LostReason>> = {
  "edit.lock_lost": "ran-out",
  "edit.taken_over": "taken-over",
  "edit.version_changed": "version-changed",
};

const lostReasonsByCode: ReadonlyMap<string, LostReason> = new Map(Object.entries(lostReasons));

/** Why a save was refused for the lock's sake, from the code of the error; nothing for any other error. */
export function lostReasonOf(code: ErrorCode | undefined): LostReason | undefined {
  return code === undefined ? undefined : lostReasonsByCode.get(code);
}

/** Whether taking the lock again can work after this loss: the time ran out or someone took over; a text that changed meanwhile needs more than the lock. */
export function lockCanBeRetaken(reason: LostReason): boolean {
  return reason === "ran-out" || reason === "taken-over";
}

/** The same from the code of a refusal: whether *Try again* can work, the text being kept. */
export function retakingLockHelps(code?: ErrorCode): boolean {
  const reason = lostReasonOf(code);
  return reason !== undefined && lockCanBeRetaken(reason);
}

/** The newest of the drafts that differ from the saved text, or nothing. */
export function newestDraft(
  found: readonly (OfferedDraft | null)[],
  saved: string,
): OfferedDraft | undefined {
  return found
    .filter((draft): draft is OfferedDraft => draft !== null && draft.text !== saved)
    .toSorted((a, b) => Date.parse(b.savedAt) - Date.parse(a.savedAt))[0];
}

/** The state of a text that nobody edits: the saved text, in the box. */
export function startingState(source: {
  readonly saved: string;
  readonly version: number;
}): Editing {
  return {
    phase: { kind: "reading" },
    text: source.saved,
    saved: source.saved,
    startedFrom: source.saved,
    version: source.version,
    offered: null,
    asked: null,
    problem: null,
  };
}
