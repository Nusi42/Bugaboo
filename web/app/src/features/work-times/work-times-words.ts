import { t } from "@bugaboo/messages";

import { weekDaysOf, weekdayName, weekFirstOf } from "../../shared/clock.ts";
import { formatCalendarDay } from "../../shared/format.ts";
import { listText } from "../../shared/list-text.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { type FirstDay } from "../../shared/region.ts";
import type { Away, UsualDay, WorkTimes } from "./work-times.ts";

/** A Monday on the calendar, to name the weekdays by: the server sends the usual days Monday first. */
const A_MONDAY = "2024-01-01";

/** One weekday of the usual work days, in the order the person's week starts. */
export interface Weekday {
  /** Where the day stands in the usual days of the server, which start on Monday. */
  readonly at: number;
  /** A date that falls on the weekday, for its name. */
  readonly date: string;
}

/** The seven weekdays in the order the person's *Region* starts the week (UX §4.13). */
export function weekOf(firstDay: FirstDay): readonly Weekday[] {
  const days = weekDaysOf(weekFirstOf(A_MONDAY, firstDay));
  const monday = days.indexOf(A_MONDAY);
  return days.map((date, at) => ({ at: (at - monday + days.length) % days.length, date }));
}

/** The ticked days as runs of days that follow each other: `Mon–Fri`, `Mon, Wed–Thu`. */
function runsOf(week: readonly Weekday[], usual: readonly UsualDay[]): string {
  const runs: (readonly Weekday[])[] = [];
  let run: Weekday[] = [];
  for (const day of week) {
    if (usual[day.at]?.works === true) {
      run.push(day);
    } else if (run.length > 0) {
      runs.push(run);
      run = [];
    }
  }
  if (run.length > 0) {
    runs.push(run);
  }
  return runs
    .map((days) => {
      const first = days[0];
      const last = days.at(-1);
      if (first === undefined || last === undefined) {
        return "";
      }
      const name = weekdayName(first.date, "short");
      return first === last
        ? name
        : t("work-times.day-run", { first: name, last: weekdayName(last.date, "short") });
    })
    .join(", ");
}

/**
 * The line above the usual work days: "Mon–Fri, 8h a day" when the days have the same hours,
 * "Mon–Thu, Sat, 36h a week" when they do not, and that there are none.
 */
export function usualSummary(week: readonly Weekday[], usual: readonly UsualDay[]): string {
  const worked = usual.filter((day) => day.works);
  const first = worked[0];
  if (first === undefined) {
    return t("work-times.usual-none");
  }
  const days = runsOf(week, usual);
  return worked.every((day) => day.minutes === first.minutes)
    ? t("work-times.usual-a-day", { days, length: minutesText(first.minutes) })
    : t("work-times.usual-a-week", {
        days,
        length: minutesText(worked.reduce((sum, day) => sum + day.minutes, 0)),
      });
}

/** The days of an away time: "26.10.2026 – 30.10.2026", "12.11.2026, afternoon". */
export function awayDaysText(
  away: Pick<Away, "first" | "last" | "first_afternoon_only" | "last_morning_only">,
): string {
  const first = formatCalendarDay(away.first);
  const last = formatCalendarDay(away.last);
  const firstSaid = away.first_afternoon_only
    ? t("work-times.day-afternoon", { date: first })
    : first;
  if (away.first === away.last) {
    return away.last_morning_only ? t("work-times.day-morning", { date: first }) : firstSaid;
  }
  const lastSaid = away.last_morning_only ? t("work-times.day-morning", { date: last }) : last;
  return t("work-times.days", { first: firstSaid, last: lastSaid });
}

/** The next public holidays that count, as the line under *Holidays* says them; that none count. */
export function nextHolidaysText(workTimes: Pick<WorkTimes, "next_holidays">): string {
  return workTimes.next_holidays.length === 0
    ? t("work-times.holidays-none")
    : t("work-times.holidays-next", {
        days: listText(
          workTimes.next_holidays.map((day) =>
            t("work-times.holiday-on", {
              date: formatCalendarDay(day.date),
              holiday: t(`holiday.${day.holiday}`),
            }),
          ),
        ),
      });
}
