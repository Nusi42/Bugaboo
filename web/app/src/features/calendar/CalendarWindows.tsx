import type { CalendarEntry } from "./calendar-data.ts";
import { ChangeDayWindow } from "./ChangeDayWindow.tsx";
import { DayListWindow } from "./DayListWindow.tsx";

export interface CalendarWindowsProps {
  /** The issue whose day is being changed with *Change due date …*. */
  readonly changing: CalendarEntry | undefined;
  readonly onCloseChange: () => void;
  /** The day whose whole list is open (*+5 more*). */
  readonly openDay: string | undefined;
  readonly onCloseDay: () => void;
}

/** The windows that open over the calendar: the box of a day, and the list of a day. */
export function CalendarWindows({
  changing,
  onCloseChange,
  openDay,
  onCloseDay,
}: CalendarWindowsProps) {
  return (
    <>
      {changing === undefined ? null : <ChangeDayWindow entry={changing} onClose={onCloseChange} />}
      {openDay === undefined ? null : <DayListWindow day={openDay} onClose={onCloseDay} />}
    </>
  );
}
