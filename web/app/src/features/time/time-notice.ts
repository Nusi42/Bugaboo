import { useSyncExternalStore } from "react";

import { createStore } from "../../shared/store.ts";

/** How loud a notice is: good news, a hint, or a problem. */
export type NoticeTone = "success" | "info" | "danger";

/** What the timer part tells the person after something happened: one sentence. */
export interface TimeNotice {
  readonly text: string;
  readonly tone: NoticeTone;
}

const current = createStore<TimeNotice | null>(null);

/** Tells the person one sentence in the timer part; a newer one replaces the older. */
export function tellTime(text: string, tone: NoticeTone = "info"): void {
  current.replace({ text, tone });
}

/** Takes the notice away. */
export function forgetNotice(): void {
  current.replace(null);
}

/** The notice now, if there is one. */
export function useTimeNotice(): TimeNotice | null {
  return useSyncExternalStore(current.subscribe, current.snapshot);
}
