/**
 * Times of day and weeks as the timesheet types and draws them: the reader of a time of day typed
 * into a cell of the work day, the moment a time of day on a day in a zone is, and the seven days
 * of a person's week. Dates and clocks are written here and in `format.ts` only (TXT-DATES).
 */
import { locale, t } from "@bugaboo/messages";

import {
  addDays,
  calendarDayOf,
  clockPartsOf,
  dayNumbersOf,
  formatCalendarDay,
  timeOfDayText,
} from "./format.ts";
import { currentRegion, deviceTimeZone, type FirstDay, type Region } from "./region.ts";

const minutesPerHour = 60;
const hoursPerDay = 24;
const hoursPerHalfDay = 12;
const daysPerWeek = 7;
const msPerMinute = 60_000;

/**
 * `8`, `17`, `8:05`, `8.05`, `16h30`, `805`, `0805`, `5:15 pm`, `5.15 p.m.`: hours, then optional minutes
 * (with a colon, a point, an `h` or nothing between), then optional am or pm.
 */
const CLOCK = /^(\d{1,2})(?:[:.h]?(\d{2}))?\s*(?:([ap])\.?m\.?)?$/iu;

/**
 * A time of day a person typed, as minutes after midnight, or `undefined` when it reads as none.
 * Both clocks are read whatever the person's *Time of day* is: somebody who chose 24 hours may type
 * `5 pm`, and somebody who chose 12 hours may type `17:15`.
 */
export function readClock(text: string): number | undefined {
  const typed = text.trim();
  const found = CLOCK.exec(typed);
  if (found === null) {
    return undefined;
  }
  const hour = Number(found[1]);
  const minute = found[2] === undefined ? 0 : Number(found[2]);
  const half = found[3] === undefined ? undefined : `${found[3].toLowerCase()}m`;
  if (minute >= minutesPerHour) {
    return undefined;
  }
  if (half === undefined) {
    return hour < hoursPerDay ? hour * minutesPerHour + minute : undefined;
  }
  if (hour < 1 || hour > hoursPerHalfDay) {
    return undefined;
  }
  const hours = (hour % hoursPerHalfDay) + (half === "pm" ? hoursPerHalfDay : 0);
  return hours * minutesPerHour + minute;
}

/**
 * A time of day for a sentence that shows what to type (`8:30`, or `8:30 AM` for a person on the
 * 12-hour clock): the example is written on the person's own clock.
 */
export function exampleClock(hour: number, minute: number): string {
  return timeOfDayText(hour, minute, currentRegion());
}

/** The zone a person's day is counted in: `device` is the zone of this device. */
export function zoneNameOf(timeZone: string): string {
  return timeZone === "device" ? deviceTimeZone() : timeZone;
}

/** The region `clockPartsOf` needs when only the zone matters. */
const zoneRegion = {
  timeZone: "UTC",
  dateFormat: "year-month-day",
  timeOfDay: "24-hour",
  numberFormat: "point",
  firstDay: "monday",
} as const;

/**
 * The zone a day with no record yet starts in, as the server decides it (`zone_for` of
 * `working_hours::change`): the zone the person chose, else the one of the device. A person who
 * chose UTC is counted as having chosen none, because UTC is also where nothing is chosen.
 */
export function zoneOfNewDay(timeZone: string, device: string = deviceTimeZone()): string {
  return timeZone === "device" || timeZone === "UTC" ? device : timeZone;
}

/**
 * The date of today for the person: the day the server starts their work day on, in the zone of
 * their region (`zoneOfNewDay`). The one "today" of the sidebar, the timesheet, *Add time* and
 * *Change start time*.
 */
export function personToday(region: Region, now: number): string {
  return calendarDayOf(now, { ...region, timeZone: zoneOfNewDay(region.timeZone) });
}

/**
 * The moment (UTC, RFC 3339) when the clock of `timeZone` shows `minutes` after midnight on `day`.
 * A time the clocks skip (the hour of the change to summer time) is the moment after the jump.
 */
export function momentOfClock(day: string, minutes: number, timeZone: string): string {
  const zone = { timeZone: zoneNameOf(timeZone) };
  const [year, month, date] = dayNumbersOf(day);
  const wanted = Date.UTC(year, month - 1, date, 0, minutes);
  let guess = wanted;
  // The offset of the zone at the guess moves the guess; a second round settles it across a change.
  for (let round = 0; round < 2; round += 1) {
    const clock = clockPartsOf(guess, { ...zoneRegion, ...zone });
    const shown = Date.UTC(clock.year, clock.month - 1, clock.day, clock.hour, clock.minute);
    guess += wanted - shown;
  }
  return new Date(guess).toISOString();
}

/** The minutes after midnight that the clock of `timeZone` shows at `moment`. */
export function clockMinutesOf(moment: string, timeZone: string): number {
  const clock = clockPartsOf(new Date(moment).getTime(), {
    ...zoneRegion,
    timeZone: zoneNameOf(timeZone),
  });
  return clock.hour * minutesPerHour + clock.minute;
}

/** The weekday number (0 for Sunday) of a day on the calendar. */
function weekdayOf(day: string): number {
  const [year, month, date] = dayNumbersOf(day);
  return new Date(Date.UTC(year, month - 1, date)).getUTCDay();
}

/** The weekday each first day of the week is (0 for Sunday); *device* counts from Monday, like the server. */
const firstWeekday: Readonly<Record<FirstDay, number>> = {
  device: 1,
  monday: 1,
  sunday: 0,
  saturday: 6,
};

/** The first day of the week that `day` is in. */
export function weekFirstOf(day: string, firstDay: FirstDay): string {
  const back = (weekdayOf(day) - firstWeekday[firstDay] + daysPerWeek) % daysPerWeek;
  return addDays(day, -back);
}

/** The seven days of the week that starts on `first`. */
export function weekDaysOf(first: string): readonly string[] {
  return Array.from({ length: daysPerWeek }, (_, at) => addDays(first, at));
}

/** The minutes between two moments, rounded to whole minutes. */
export function minutesBetween(from: string, to: string): number {
  return Math.round((new Date(to).getTime() - new Date(from).getTime()) / msPerMinute);
}

/** The short or long name of the weekday of a day on the calendar (`Mon`, `Monday`), in the language of the app. */
export function weekdayName(day: string, style: "short" | "long"): string {
  return new Intl.DateTimeFormat(locale, { weekday: style, timeZone: "UTC" }).format(
    new Date(`${day}T00:00:00Z`),
  );
}

/**
 * A day as a title or a line says it: its weekday and its date (`Thursday, 10/08/2026`). The one way
 * it is written, on a computer and on a phone, in a title and in the text of a window.
 */
export function weekdayAndDate(day: string, region: Region): string {
  return t("common.weekday-date", {
    weekday: weekdayName(day, "long"),
    date: formatCalendarDay(day, region),
  });
}

/** The number of a day in its month (`2026-10-05` is 5). */
export function dayOfMonth(day: string): number {
  return dayNumbersOf(day)[2];
}
