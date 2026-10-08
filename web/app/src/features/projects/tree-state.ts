import { readStoredJson, storageKey, writeStored } from "@bugaboo/ui";
import { useSyncExternalStore } from "react";

/** Remembers in this browser which projects of the tree the person opened. */
function read(): ReadonlySet<string> {
  const stored = readStoredJson(storageKey.projectTree);
  return new Set(
    Array.isArray(stored) ? stored.filter((id): id is string => typeof id === "string") : [],
  );
}

let opened: ReadonlySet<string> = read();
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Whether the person has the sub-projects of this project open in the tree. */
export function useOpened(project: string): boolean {
  return useSyncExternalStore(subscribe, () => opened.has(project));
}

/** Opens or closes a project in the tree, and remembers it. */
export function toggleOpened(project: string): void {
  const next = new Set(opened);
  if (!next.delete(project)) {
    next.add(project);
  }
  opened = next;
  writeStored(storageKey.projectTree, JSON.stringify([...next]));
  for (const listener of listeners) {
    listener();
  }
}
