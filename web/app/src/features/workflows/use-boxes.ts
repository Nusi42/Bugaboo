import { useCallback, useRef, useState } from "react";

import { useOnResize } from "../../shared/use-on-resize.ts";
import type { Box } from "./drawing-layout.ts";

/** The boxes of the drawing by the ID of the status, measured from the top left corner of the drawing. */
export type Placed = Readonly<Record<string, Box>>;

/** The ID of the element of a status in the page, which the measure finds it by. */
export function boxId(prefix: string, status: string): string {
  return `${prefix}-${status}`;
}

/**
 * The boxes of the drawing as they stand now. `statuses` are the IDs of the statuses in the draft;
 * their elements are found in the page by `boxId`. The boxes are measured whenever the statuses
 * change and when the drawing changes size, and a measure that finds nothing new changes nothing.
 */
export function useBoxes(prefix: string, statuses: readonly string[]) {
  const frame = useRef<HTMLDivElement>(null);
  const [placed, setPlaced] = useState<Placed>({});
  const measure = useCallback((): void => {
    const origin = frame.current?.getBoundingClientRect();
    if (origin === undefined) {
      return;
    }
    const next: Record<string, Box> = {};
    for (const id of statuses) {
      const found = document
        .querySelector(`#${CSS.escape(boxId(prefix, id))}`)
        ?.getBoundingClientRect();
      if (found !== undefined) {
        next[id] = {
          left: found.left - origin.left,
          top: found.top - origin.top,
          width: found.width,
          height: found.height,
        };
      }
    }
    setPlaced((before) => (JSON.stringify(before) === JSON.stringify(next) ? before : next));
  }, [prefix, statuses]);
  const watched = useCallback(() => [frame.current], []);
  useOnResize(measure, watched);
  return { frame, placed };
}
