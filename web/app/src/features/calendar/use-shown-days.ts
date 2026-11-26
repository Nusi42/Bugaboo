import { calendarDaysMax } from "@bugaboo/api-client";
import { useState } from "react";

import { personToday } from "../../shared/clock.ts";
import { addDays } from "../../shared/format.ts";
import { useRegion } from "../../shared/region.ts";
import { useNow } from "../../shared/use-now.ts";
import type { CalendarInput } from "./calendar-input.ts";
import { steppedAnchor, windowOf, type Window } from "./span-days.ts";

/** The days a phone lists: from today on. */
function phoneWindow(today: string): Window {
  return { weeks: [], first: today, last: addDays(today, calendarDaysMax - 1), month: undefined };
}

/**
 * The days on screen. They are the screen's, not the view's (a saved search opens around the day it is
 * opened): *Earlier* and *Later* step them, *Today* brings them back.
 */
export function useShownDays({ phone, view }: Pick<CalendarInput, "phone" | "view">) {
  const region = useRegion();
  const today = personToday(region, useNow());
  const [anchor, setAnchor] = useState(today);
  const { span } = view.calendar;
  return {
    today,
    window: phone ? phoneWindow(today) : windowOf(anchor, span, region.firstDay),
    onStep: (direction: -1 | 1): void => {
      setAnchor(steppedAnchor(anchor, span, direction));
    },
    onToday: (): void => {
      setAnchor(today);
    },
  };
}
