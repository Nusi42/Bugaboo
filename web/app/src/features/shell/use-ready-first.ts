import { useState, type KeyboardEvent } from "react";

import { firstReady, type ResultGroup } from "./quick-search-collect.ts";

/** What the box needs to make the first result the one that is ready to open. */
export interface ReadyFirst {
  /** Whether the first result is the ready one: no arrow key was pressed since the words changed. */
  readonly firstIsReady: boolean;
  /** To be called when the words change: the first result is the ready one again. */
  readonly wordsChanged: () => void;
  /** For the key presses of the whole dialog, before the box and the list see them. */
  readonly onKeyDownCapture: (event: KeyboardEvent) => void;
}

/**
 * Until the person presses an arrow key the first result is the one that is ready to open, also
 * when a group that comes later puts another one on top; after that the list stays where they put
 * it. Enter opens the ready one; once an arrow key was pressed the list opens the one that has the
 * focus.
 */
export function useReadyFirst(
  groups: readonly ResultGroup[],
  open: (key: string) => void,
): ReadyFirst {
  const [moved, setMoved] = useState(false);
  return {
    firstIsReady: !moved,
    wordsChanged: () => {
      setMoved(false);
    },
    onKeyDownCapture: (event) => {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        setMoved(true);
      }
      const ready = moved ? undefined : firstReady(groups);
      if (event.key === "Enter" && ready !== undefined) {
        event.preventDefault();
        event.stopPropagation();
        open(ready);
      }
    },
  };
}
