import type { Packer } from "@bugaboo/wasm";
import { useState, type KeyboardEvent } from "react";

import { dragKinds } from "../../shared/drag-kinds.ts";
import { grownBy, samePlaces, steppedBy, type GridPlace, type Step } from "./grid-layout.ts";
import type { Gesture, Placing } from "./use-grid-drag.ts";

/** The arrow keys, by the step each takes. */
const steps: Readonly<Record<string, Step>> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
};

/** The keys that end *Move …* or *Resize …*. */
const keep = "Enter";
const back = "Escape";

export interface BoxKeysInput {
  readonly places: readonly GridPlace[];
  readonly pack: Packer | null;
  /** *Change* is on: when it ends, placing ends with it. */
  readonly changing: boolean;
  /** The places are kept: the same as letting a dragged box go. */
  readonly onDrop: (placing: Placing) => void;
}

/** What one key does to a box being placed: the next places, or the end of placing (kept or not). */
function afterKey(
  key: string,
  placing: Placing,
  pack: Packer,
): { readonly next: Placing } | { readonly end: "keep" | "back" } | undefined {
  const step = steps[key];
  if (step !== undefined) {
    const places =
      placing.gesture === dragKinds.dashboardBox.kind
        ? steppedBy(placing.places, placing.at, step, pack)
        : grownBy(placing.places, placing.at, step, pack);
    return { next: { ...placing, places } };
  }
  if (key === keep || key === back) {
    return { end: key === keep ? "keep" : "back" };
  }
  return undefined;
}

/** Whether a key press is one placing reads: no modifier with it, and no letter being composed. */
function isPlain(event: KeyboardEvent<HTMLElement>): boolean {
  return (
    !event.altKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    !event.shiftKey &&
    !event.nativeEvent.isComposing
  );
}

/**
 * *Move …* and *Resize …* of the menu of a box (UX §4.10, TEST-A11Y): the typed way of each drag. While one is on, the
 * handle of the box has the focus and reads the arrow keys: they move the shadow of the box (a column sideways, past the
 * box above or below) or change its size, Enter keeps it and Escape goes back, so the keys do exactly what a drag does.
 * Leaving the box, or the end of *Change*, goes back too.
 */
export function useBoxKeys({ places, pack, changing, onDrop }: BoxKeysInput): {
  readonly placing: Placing | null;
  readonly start: (at: number, gesture: Gesture) => void;
  readonly onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
  readonly onBlur: (stillInBox: boolean) => void;
} {
  const [placing, setPlacing] = useState<Placing | null>(null);
  const shown = changing ? placing : null;
  return {
    placing: shown,
    start: (at, gesture) => {
      setPlacing({ at, gesture, places });
    },
    onKeyDown: (event) => {
      if (shown === null || pack === null || !isPlain(event)) {
        return;
      }
      const after = afterKey(event.key, shown, pack);
      if (after === undefined) {
        return;
      }
      event.preventDefault();
      if ("next" in after) {
        setPlacing(after.next);
        return;
      }
      setPlacing(null);
      if (after.end === "keep" && !samePlaces(shown.places, places)) {
        onDrop(shown);
      }
    },
    // The menu that chose *Move …* gives the focus back to its own button in the box, which the handle takes again;
    // the focus going anywhere else ends placing.
    onBlur: (stillInBox) => {
      if (!stillInBox) {
        setPlacing(null);
      }
    },
  };
}
