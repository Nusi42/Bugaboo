import { api } from "./api.ts";
import { messageOf } from "./errors.ts";
import {
  isMySettings,
  mySettingsKey,
  withChange,
  type MySettings,
  type SettingChange,
} from "./my-settings.ts";
import { optimisticSend } from "./optimistic.ts";
import { queryClient } from "./query-client.ts";

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

/** Sends one change: the value counts at once, and goes back when the server refuses. */
function send(change: SettingChange): Promise<void> {
  const path = { params: { path: { key: change.key } } };
  return optimisticSend(() => queryClient, {
    queryKey: mySettingsKey(),
    isCached: isMySettings,
    apply: (before: MySettings | undefined) =>
      before === undefined ? undefined : withChange(before, change),
    send: async (changed: SettingChange) => {
      if (changed.value === null) {
        await api.client.DELETE("/api/v1/me/settings/{key}", path);
        return;
      }
      await api.client.PUT("/api/v1/me/settings/{key}", {
        ...path,
        body: { value: changed.value },
      });
    },
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
