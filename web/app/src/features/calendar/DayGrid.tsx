import { part, partOf } from "@bugaboo/ui";

import { weekdayName } from "../../shared/clock.ts";
import { spanOf, type SpanWord } from "../../shared/list-view.ts";
import { DayCell } from "./DayCell.tsx";
import { inMonth, type Window } from "./span-days.ts";

export interface DayGridProps {
  readonly window: Window;
  readonly span: SpanWord;
}

/** How many lines of issues and marks fit in a day of the month and of the week before *+5 more*. */
const linesIn: Readonly<Record<SpanWord, number>> = { month: 3, week: 8 };

/**
 * The days of a month or a week as a grid of seven columns (UX §4.16): the weekdays above in the
 * order the person's *Region* starts the week with, and a day after the other under them. A month
 * draws the days around it quieter.
 */
export function DayGrid({ window, span }: DayGridProps) {
  const [first = []] = window.weeks;
  return (
    <div {...partOf(part.calendarGrid, { variants: [span] })}>
      {span === spanOf.week
        ? null
        : first.map((day) => (
            <span key={day} {...partOf(part.calendarWeekday)} aria-hidden>
              {weekdayName(day, "short")}
            </span>
          ))}
      {window.weeks.flat().map((day) => (
        <DayCell
          key={day}
          day={day}
          outside={!inMonth(window, day)}
          lines={linesIn[span]}
          named={span === spanOf.week}
        />
      ))}
    </div>
  );
}
