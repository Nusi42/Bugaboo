import { readStoredJson, storageKey, writeStored } from "@bugaboo/ui";
import { useSyncExternalStore } from "react";

import { createStore } from "../../shared/store.ts";

/** Remembers in this browser which projects of the tree the person opened. */
function read(): ReadonlySet<string> {
  const stored = readStoredJson(storageKey.projectTree);
  return new Set(
    Array.isArray(stored) ? stored.filter((id): id is string => typeof id === "string") : [],
  );
}

const opened = createStore<ReadonlySet<string>>(read());

/** Whether the person has the sub-projects of this project open in the tree. */
export function useOpened(project: string): boolean {
  return useSyncExternalStore(opened.subscribe, () => opened.snapshot().has(project));
}

/** Opens or closes a project in the tree, and remembers it. */
export function toggleOpened(project: string): void {
  const next = new Set(opened.snapshot());
  if (!next.delete(project)) {
    next.add(project);
  }
  writeStored(storageKey.projectTree, JSON.stringify([...next]));
  opened.replace(next);
}
