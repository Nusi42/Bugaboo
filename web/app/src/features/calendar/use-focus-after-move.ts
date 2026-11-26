import { useEffect, useRef } from "react";

import { idOf, type CalendarEntry } from "./calendar-data.ts";
import type { PickUp } from "./use-pick-up.ts";

/** The button of an issue that has the focus again once the issue stands on its new day. */
export function useFocusAfterMove(
  entry: CalendarEntry,
  pick: Pick<PickUp, "focusAfter" | "focused">,
) {
  const ref = useRef<HTMLButtonElement>(null);
  const { focusAfter, focused } = pick;
  const mine = focusAfter === idOf(entry);
  useEffect(() => {
    if (mine) {
      ref.current?.focus();
      focused();
    }
  }, [mine, focused]);
  return ref;
}
