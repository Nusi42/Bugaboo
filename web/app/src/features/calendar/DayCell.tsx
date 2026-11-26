import { t } from "@bugaboo/messages";
import { ariaCurrent, Button, buttonLook, part, partOf } from "@bugaboo/ui";
import { useRef } from "react";

import { dayOfMonth, weekdayAndDate, weekdayName } from "../../shared/clock.ts";
import { useRegion } from "../../shared/region.ts";
import { useCalendar } from "./calendar-context.ts";
import { DayLines } from "./DayLines.tsx";
import { useLinesOf } from "./use-lines-of.ts";
import { useDropOnDay } from "./use-drop-on-day.ts";

export interface DayCellProps {
  readonly day: string;
  /** The day is outside the month the calendar is of: drawn quieter. */
  readonly outside: boolean;
  /** How many lines of issues and marks fit before *+5 more*. */
  readonly lines: number;
  /** The week calendar names the weekday in each day; the month has a row of them above. */
  readonly named: boolean;
}

/** The number of the day, with its weekday where the days stand in a week and no row above names it. */
function DayNumber({ day, named }: Pick<DayCellProps, "day" | "named">) {
  return (
    <span {...partOf(part.calendarDayNumber)} aria-hidden>
      {named
        ? t("calendar.week-day", { weekday: weekdayName(day, "short"), day: dayOfMonth(day) })
        : dayOfMonth(day)}
    </span>
  );
}

/**
 * One day (UX §4.16): its number, the marks and issues on it up to the lines that fit, and *+5 more*
 * for the rest. Today is marked; an issue dragged over it shows that it takes it. Every day is as high
 * as the others of its calendar, so a drop moves nothing but the issue (UX §6.4).
 */
export function DayCell({ day, outside, lines, named }: DayCellProps) {
  const { model, onOpenDay } = useCalendar();
  const name = weekdayAndDate(day, useRegion());
  const ref = useRef<HTMLElement>(null);
  const drop = useDropOnDay(day, ref, model);
  const total = useLinesOf(day).length;
  const today = day === model.today;
  const over = drop.isDropTarget || model.pick.picked?.day === day;
  return (
    <article
      ref={ref}
      {...drop.dropProps}
      aria-label={name}
      aria-current={today ? ariaCurrent.date : undefined}
      {...partOf(part.calendarDay, {
        states: [today && "today", outside && "outside", over && "over"],
      })}
    >
      <DayNumber day={day} named={named} />
      <DayLines day={day} lines={lines} />
      {total > lines ? (
        <div {...partOf(part.calendarDayMore)}>
          <Button
            look={buttonLook.quiet}
            aria-label={t("calendar.more-label", { count: total - lines, day: name })}
            onPress={() => {
              onOpenDay(day);
            }}
          >
            {t("calendar.more", { count: total - lines })}
          </Button>
        </div>
      ) : null}
    </article>
  );
}
