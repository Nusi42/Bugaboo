import { ApiError, type SettingKey } from "@bugaboo/api-client";

import { api } from "./api.ts";
import { messageOf } from "./errors.ts";
import {
  currentMySettings,
  isMySettings,
  mySettingsKey,
  settingOf,
  type MySettings,
  type Setting,
  type SettingValue,
} from "./my-settings.ts";
import { optimisticSend } from "./optimistic.ts";
import { queryClient } from "./query-client.ts";
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

/**
 * The newest version of each setting that an answer of the server named. The page reads the
 * setting again after a change, but not at once, so the next change is made from the version the
 * one before made. A version that is too old or too new (another tab reset the setting, or
 * another person signed in) is refused as stale and the setting is read again, so a wrong guess
 * costs one request and never overwrites anything.
 */
const versionsSeen = new Map<string, number>();

/** Forgets the versions seen: when the person leaves, the next person starts without them. */
export function forgetSeenVersions(): void {
  versionsSeen.clear();
}

/** The version a change is made from: the newest one this page knows of the setting. */
function versionToSend(key: SettingKey): number | undefined {
  const cached = settingOf(currentMySettings(), key)?.version;
  const seen = versionsSeen.get(key);
  return cached === undefined || cached === null ? seen : Math.max(cached, seen ?? cached);
}

function remember(key: SettingKey, version: number | null | undefined): void {
  if (typeof version === "number") {
    versionsSeen.set(key, version);
  }
}

/** Whether the server refused because the setting was changed since the version that was sent. */
function isStale(error: unknown): boolean {
  return error instanceof ApiError && error.problem?.code === "edit.version-changed";
}

/** The setting as the server holds it now. */
async function readNow(key: SettingKey): Promise<Setting | undefined> {
  const { data } = await api.client.GET("/api/v1/me/settings", {
    params: { query: { topic: key } },
  });
  return settingOf(data, key);
}

/**
 * Asks the server for one change, made from `version` (the one the page knows; nothing when it
 * knows none). When another tab or device changed the setting meanwhile, the setting is read again
 * and only this change is made on what is found there (`rebase`; without it the value of the
 * change), so a change of one part of a value never undoes another part set elsewhere (M5-F97).
 * It reads again at most {@link rereadsMax} times.
 */
async function putChange(
  change: SettingChange,
  value: SettingValue,
  version: number | undefined,
  rereads: number,
): Promise<void> {
  try {
    const { data } = await api.client.PUT("/api/v1/me/settings/{key}", {
      params: { path: { key: change.key } },
      body: { value, ...(version === undefined ? {} : { base_version: version }) },
    });
    remember(change.key, data?.version);
  } catch (error) {
    if (!isStale(error) || rereads >= rereadsMax) {
      throw error;
    }
    const now = await readNow(change.key);
    const again =
      change.rebase === undefined || now === undefined ? value : change.rebase(now.value);
    await putChange(change, again, now?.version ?? 0, rereads + 1);
  }
}

/** Asks the server for one change of a setting: to a new value, or to take the value away. */
async function sendChange(change: SettingChange): Promise<void> {
  if (change.value === null) {
    await api.client.DELETE("/api/v1/me/settings/{key}", {
      params: { path: { key: change.key } },
    });
    versionsSeen.delete(change.key);
    return;
  }
  await putChange(change, change.value, versionToSend(change.key), 0);
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
