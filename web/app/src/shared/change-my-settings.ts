import { ApiError, conflictOf, type SettingKey } from "@bugaboo/api-client";

import { api } from "./api.ts";
import { messageOf } from "./errors.ts";
import {
  currentMySettings,
  isMySettings,
  isSettingKey,
  mySettingsKey,
  settingOf,
  type MySettings,
  type SettingValue,
} from "./my-settings.ts";
import { optimisticSend } from "./optimistic.ts";
import { onServerRead, queryClient } from "./query-client.ts";
import { withChange, type SettingChange } from "./setting-change.ts";

/** What the person is told about their changes: nothing before the first, then where it stands. */
export interface SaveProgress {
  readonly state: "idle" | "saving" | "saved" | "not-saved";
  /** The plain message when the server refused. */
  readonly problem: string | null;
}

let progress: SaveProgress = { state: "idle", problem: null };
let running = 0;
let refused: string | null = null;
const watchers = new Set<() => void>();

function report(next: SaveProgress): void {
  progress = next;
  for (const watcher of watchers) {
    watcher();
  }
}

/** Where the person's changes stand (for *Saving…*, *Saved* and *Not saved yet*). */
export const saveProgress = {
  read: (): SaveProgress => progress,
  subscribe: (watcher: () => void): (() => void) => {
    watchers.add(watcher);
    return () => {
      watchers.delete(watcher);
    };
  },
};

/** Tells the person that a change stays in this page only: the server has not got it. */
export function reportNotSaved(): void {
  report({ state: "not-saved", problem: null });
}

/**
 * The last request on its way for each setting. The server keeps whatever arrives last, and two
 * requests sent together may arrive in either order, so a setting's requests go one after the other.
 */
const tails = new Map<string, Promise<unknown>>();

/** Runs `request` after every request of the same setting that was asked for before it is done. */
export function inOrderOfKey<Answer>(key: string, request: () => Promise<Answer>): Promise<Answer> {
  const run = (tails.get(key) ?? Promise.resolve()).then(request, request);
  const tail = run.then(forget, forget);
  function forget(): void {
    if (tails.get(key) === tail) {
      tails.delete(key);
    }
  }
  tails.set(key, tail);
  return run;
}

/**
 * How often a change reads the setting again after the server said it was changed meanwhile. Two
 * tabs that change it at the same moment can beat each other, so it is more than once; a change
 * that still loses after that is not saved, and the person is told.
 */
export const rereadsMax = 3;

/** What the server held of a setting the last time it said so, and the version of it. */
interface Confirmed {
  readonly value: SettingValue;
  readonly version: number;
}

/**
 * The last value and version of each setting that an answer of the server named (the answer to a
 * change, or the refusal of a stale one). The page reads the setting again after a change, but not
 * at once, so a change that waits behind another is made on what the server holds after that one,
 * not on what the page drew before it (M5-F97). A version that is too old or too new (another
 * tab changed the setting, or another person signed in) is refused as stale and the change is made
 * again on what the refusal names, so a wrong guess costs one request and overwrites nothing.
 */
const confirmed = new Map<SettingKey, Confirmed>();

/** Forgets what the server confirmed: when the person leaves, the next person starts without it. */
export function forgetConfirmedSettings(): void {
  confirmed.clear();
  readLast.clear();
}

function remember(key: SettingKey, value: SettingValue, version: number | null | undefined): void {
  if (typeof version === "number") {
    confirmed.set(key, { value, version });
  }
}

/** The value of a refusal that names it, as a value of a setting. */
function valueIn(found: unknown): SettingValue | undefined {
  return typeof found === "string" || typeof found === "number" || typeof found === "boolean"
    ? found
    : undefined;
}

/**
 * What the server holds of the setting, from the refusal of a stale change: its version and value
 * (the standard value when the person has none of their own). Nothing when the refusal does not
 * say, and the setting has to be read.
 */
function heldBy(error: unknown, key: SettingKey): Confirmed | undefined {
  if (!(error instanceof ApiError) || error.problem?.code !== "edit.version-changed") {
    return undefined;
  }
  const conflict = conflictOf(error);
  const standard = settingOf(currentMySettings(), key)?.default;
  const [field] = conflict?.fields ?? [];
  const value = field === undefined ? undefined : valueIn(conflict?.current[field]);
  const held = value ?? standard;
  return typeof conflict?.version === "number" && held !== undefined
    ? { value: held, version: conflict.version }
    : undefined;
}

/** Whether the server refused because the setting was changed since the version that was sent. */
function isStale(error: unknown): boolean {
  return error instanceof ApiError && error.problem?.code === "edit.version-changed";
}

/** The setting as the server holds it now. */
async function readNow(key: SettingKey): Promise<Confirmed | undefined> {
  const { data } = await api.client.GET("/api/v1/me/settings", {
    params: { query: { topic: key } },
  });
  const setting = settingOf(data, key);
  return setting === undefined
    ? undefined
    : { value: setting.value, version: setting.version ?? 0 };
}

/** What a change is made on: a value, and the version it was read at when the page knows it. */
interface Base {
  readonly value: SettingValue;
  readonly version?: number;
}

/**
 * The newest value and version of each setting that the page read from the server (not a value
 * the page put in its cache itself, which holds the changes on their way too). A read can land
 * while a change waits behind another, with a version another tab made.
 */
const readLast = new Map<SettingKey, Confirmed>();

onServerRead((answer) => {
  if (!isMySettings(answer)) {
    return;
  }
  for (const setting of answer.settings) {
    if (isSettingKey(setting.key) && typeof setting.version === "number") {
      readLast.set(setting.key, { value: setting.value, version: setting.version });
    }
  }
});

/**
 * What a change is made on: the newest of what the server confirmed to this page and what the page
 * read last, the change made again on that value (`rebase`; without it the value of the change)
 * and its version. When the page has read nothing from the server, the value the change was built
 * from and the version in the cache.
 */
function baseOf(change: SettingChange, value: SettingValue): Base {
  const known = confirmed.get(change.key);
  const read = readLast.get(change.key);
  const latest =
    read !== undefined && (known === undefined || read.version > known.version) ? read : known;
  if (latest !== undefined) {
    return {
      value: change.rebase === undefined ? value : change.rebase(latest.value),
      version: latest.version,
    };
  }
  const cached = settingOf(currentMySettings(), change.key)?.version;
  return typeof cached === "number" ? { value, version: cached } : { value };
}

/**
 * Asks the server for one change, made on `base`. When another tab or device changed the setting
 * meanwhile, the refusal says what it holds now and only this change is made on that (`rebase`;
 * without it the value of the change), so a change of one part of a value never undoes another
 * part set elsewhere (M5-F97). It tries again at most {@link rereadsMax} times.
 */
async function putChange(change: SettingChange, base: Base, rereads: number): Promise<void> {
  try {
    const { data } = await api.client.PUT("/api/v1/me/settings/{key}", {
      params: { path: { key: change.key } },
      body: {
        value: base.value,
        ...(base.version === undefined ? {} : { version: base.version }),
      },
    });
    remember(change.key, base.value, data?.version);
  } catch (error) {
    if (!isStale(error) || rereads >= rereadsMax) {
      throw error;
    }
    const now = heldBy(error, change.key) ?? (await readNow(change.key));
    if (now !== undefined) {
      confirmed.set(change.key, now);
    }
    const value =
      change.rebase === undefined || now === undefined ? base.value : change.rebase(now.value);
    await putChange(change, { value, version: now?.version ?? 0 }, rereads + 1);
  }
}

/** Asks the server for one change of a setting: to a new value, or to take the value away. */
async function sendChange(change: SettingChange): Promise<void> {
  if (change.value === null) {
    await api.client.DELETE("/api/v1/me/settings/{key}", {
      params: { path: { key: change.key } },
    });
    confirmed.delete(change.key);
    return;
  }
  await putChange(change, baseOf(change, change.value), 0);
}

/** Sends one change: the value counts at once, and goes back when the server refuses. */
function send(change: SettingChange): Promise<void> {
  return optimisticSend(() => queryClient, {
    queryKey: mySettingsKey(),
    isCached: isMySettings,
    apply: (before: MySettings | undefined) =>
      before === undefined ? undefined : withChange(before, change),
    send: (changed: SettingChange) => inOrderOfKey(changed.key, () => sendChange(changed)),
  })(change);
}

/**
 * Changes some of the person's own settings, all at once. Every one shows at once and is sent in
 * the background; when the server refuses one, it goes back and the person gets the plain message.
 * Nothing about it is kept in this browser.
 */
export async function changeMySettings(changes: readonly SettingChange[]): Promise<void> {
  if (changes.length === 0) {
    return;
  }
  if (running === 0) {
    refused = null;
  }
  running += changes.length;
  report({ state: "saving", problem: null });
  await Promise.all(
    changes.map(async (change) => {
      try {
        await send(change);
      } catch (error) {
        refused = messageOf(error);
      } finally {
        running -= 1;
      }
    }),
  );
  if (running === 0) {
    report(
      refused === null
        ? { state: "saved", problem: null }
        : { state: "not-saved", problem: refused },
    );
  }
}
