import { useSyncExternalStore } from "react";

/** A list that other features add to while they are on screen: the project tree, search sources. */
export interface Registry<Item extends object> {
  /** Adds an item; returns the way to take it out again. */
  readonly add: (item: Item) => () => void;
  readonly list: () => readonly Item[];
  readonly subscribe: (listener: () => void) => () => void;
}

export function createRegistry<Item extends object>(): Registry<Item> {
  const listeners = new Set<() => void>();
  let items: readonly Item[] = [];
  const changed = (next: readonly Item[]): void => {
    items = next;
    for (const listener of listeners) {
      listener();
    }
  };
  return {
    add: (item) => {
      changed([...items, item]);
      return () => {
        changed(items.filter((one) => one !== item));
      };
    },
    list: () => items,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/** The items of a registry, for a component that shows them. */
export function useRegistry<Item extends object>(registry: Registry<Item>): readonly Item[] {
  return useSyncExternalStore(registry.subscribe, registry.list);
}
