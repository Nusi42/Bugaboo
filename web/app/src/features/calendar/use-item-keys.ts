import type { KeyboardEvent } from "react";

import type { CalendarEntry } from "./calendar-data.ts";
import { stepOfKey } from "./pick-up-keys.ts";
import type { PickUp } from "./use-pick-up.ts";

/** The words of the keyboard of the standard, not text for people. */
const space = " ";
const keys = { enter: "Enter", escape: "Escape" } as const;

/**
 * What the keys do on an issue of the calendar that the person may move: *Space* picks it up, the arrow
 * keys carry it over the days, *Space* or *Enter* puts it down and *Escape* puts it back. On an issue
 * that is not picked up *Enter* opens it, as a press does. `pick` is the pick up of the screen.
 */
export function itemKeys(entry: CalendarEntry, pick: PickUp, mayMove: boolean) {
  const carried = pick.carries(entry);
  return {
    onKeyDown: (event: KeyboardEvent): void => {
      if (!mayMove) {
        return;
      }
      if (!carried) {
        if (event.key === space) {
          event.preventDefault();
          pick.pickUp(entry);
        }
        return;
      }
      const days = stepOfKey(event.key, event.shiftKey);
      if (days !== undefined) {
        event.preventDefault();
        pick.carry(days);
      } else if (event.key === space || event.key === keys.enter) {
        event.preventDefault();
        pick.putDown();
      } else if (event.key === keys.escape) {
        event.preventDefault();
        event.stopPropagation();
        pick.putBack();
      }
    },
    /** A released *Space* is no press: the issue was picked up or put down by its key going down. */
    onKeyUp: (event: KeyboardEvent): void => {
      if (mayMove && event.key === space) {
        event.preventDefault();
      }
    },
    /** The issue lost the focus while it was carried: it is put back. */
    onBlur: (): void => {
      if (carried) {
        pick.putBack();
      }
    },
  };
}
