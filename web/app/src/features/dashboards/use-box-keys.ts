import type { Packer } from "@bugaboo/wasm";
import { useEffect, useState } from "react";

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

/**
 * *Move …* and *Resize …* of the menu of a box (UX §4.10, TEST-A11Y): the typed way of each drag. While one is on, the
 * arrow keys move the shadow of the box (a column sideways, past the box above or below) or change its size, Enter keeps
 * it and Escape goes back, wherever the focus is, so the keys do exactly what a drag does.
 */
export function useBoxKeys({ places, pack, onDrop }: BoxKeysInput): {
  readonly placing: Placing | null;
  readonly start: (at: number, gesture: Gesture) => void;
} {
  const [placing, setPlacing] = useState<Placing | null>(null);
  useEffect(() => {
    if (placing === null || pack === null) {
      return () => {};
    }
    const listen = (event: KeyboardEvent): void => {
      const after = afterKey(event.key, placing, pack);
      if (after === undefined) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if ("next" in after) {
        setPlacing(after.next);
        return;
      }
      setPlacing(null);
      if (after.end === "keep" && !samePlaces(placing.places, places)) {
        onDrop(placing);
      }
    };
    window.addEventListener("keydown", listen, true);
    return () => {
      window.removeEventListener("keydown", listen, true);
    };
  }, [placing, pack, places, onDrop]);
  return {
    placing,
    start: (at, gesture) => {
      setPlacing({ at, gesture, places });
    },
  };
}
