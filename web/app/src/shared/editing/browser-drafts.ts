import { draftKeepDays, msPerDay, type DraftTarget } from "@bugaboo/api-client";
import {
  browserStorage,
  forgetStored,
  readStoredJson,
  storageKey,
  writeStored,
  type StoreGetter,
} from "@bugaboo/ui";

/**
 * The copy of a draft that the browser keeps (ISS-17, ISS-11, ADR-0019): the text a person has
 * written and not saved, so a closed tab, a lost connection or a lost sign-in does not take it.
 * The server keeps a private copy too (`saveDraft`, and the heartbeat of an edit lock); the browser
 * copy is the one that is there when the server cannot be reached.
 *
 * All drafts are kept under one key of the storage, by person and by what they are for, so one
 * person never finds the draft of another at a shared device. A draft older than the keep period
 * of the server (30 days) is not read and is dropped with the next write.
 */
export interface BrowserDraft {
  /** The text as it stood. */
  readonly text: string;
  /** The version of the record the person started from; `null` for a new record. */
  readonly baseVersion: number | null;
  /** When it was written, as a moment in time. */
  readonly savedAt: string;
}

/** What is kept under the key: the drafts by their address. */
type Drafts = Readonly<Record<string, BrowserDraft>>;

/** What a draft is for, as one text: the kind, then the record and part, or the project. */
export function addressOf(person: string, target: DraftTarget): string {
  const what =
    "record" in target
      ? `edit:${target.record.kind}:${target.record.id}:${target.part}`
      : `new:${target.kind}:${target.project}`;
  return `${person}:${what}`;
}

function isDraft(value: unknown): value is BrowserDraft {
  return (
    typeof value === "object" &&
    value !== null &&
    "text" in value &&
    typeof value.text === "string" &&
    "savedAt" in value &&
    typeof value.savedAt === "string" &&
    "baseVersion" in value &&
    (value.baseVersion === null || typeof value.baseVersion === "number")
  );
}

/** Whether a draft is still within its keep period at `nowMs`. */
function isKept(draft: BrowserDraft, nowMs: number): boolean {
  const savedAt = Date.parse(draft.savedAt);
  return !Number.isNaN(savedAt) && nowMs - savedAt < draftKeepDays * msPerDay;
}

/** The drafts that are kept now; what is not a draft, or too old, is left out. */
function readAll(nowMs: number, store: StoreGetter): Drafts {
  const found = readStoredJson(storageKey.drafts, store);
  if (typeof found !== "object" || found === null) {
    return {};
  }
  const kept: Record<string, BrowserDraft> = {};
  for (const [address, draft] of Object.entries(found)) {
    if (isDraft(draft) && isKept(draft, nowMs)) {
      kept[address] = draft;
    }
  }
  return kept;
}

/**
 * The draft of the person for a target, or `null`. Nothing is read, written or forgotten for
 * nobody (`null`: the app does not know yet who is signed in), so no draft is ever filed under no
 * person. This is the one place that rule is kept.
 */
export function readBrowserDraft(
  person: string | null,
  target: DraftTarget,
  nowMs: number = Date.now(),
  store: StoreGetter = browserStorage,
): BrowserDraft | null {
  return person === null ? null : (readAll(nowMs, store)[addressOf(person, target)] ?? null);
}

/** Keeps a text as the draft of the person for a target. */
export function writeBrowserDraft(
  person: string | null,
  target: DraftTarget,
  draft: Omit<BrowserDraft, "savedAt">,
  nowMs: number = Date.now(),
  store: StoreGetter = browserStorage,
): void {
  if (person === null) {
    return;
  }
  const all = readAll(nowMs, store);
  const saved: BrowserDraft = { ...draft, savedAt: new Date(nowMs).toISOString() };
  writeStored(
    storageKey.drafts,
    JSON.stringify({ ...all, [addressOf(person, target)]: saved }),
    store,
  );
}

/** Throws the draft of the person for a target away. */
export function forgetBrowserDraft(
  person: string | null,
  target: DraftTarget,
  nowMs: number = Date.now(),
  store: StoreGetter = browserStorage,
): void {
  if (person === null) {
    return;
  }
  const address = addressOf(person, target);
  const { [address]: _gone, ...rest } = readAll(nowMs, store);
  if (Object.keys(rest).length === 0) {
    forgetStored(storageKey.drafts, store);
  } else {
    writeStored(storageKey.drafts, JSON.stringify(rest), store);
  }
}

/** Forgets every draft this browser kept (signing out on purpose). */
export function forgetBrowserDrafts(store: StoreGetter = browserStorage): void {
  forgetStored(storageKey.drafts, store);
}
