import { useSyncExternalStore } from "react";

/** How dates are written, as the setting says it. */
export type DateFormat = "device" | "day-month-year" | "month-day-year" | "year-month-day";
/** How big numbers and decimals are written, as the setting says it. */
export type NumberFormat = "device" | "point" | "comma" | "space";
/** The day a week starts on, as the setting says it. */
export type FirstDay = "device" | "monday" | "sunday" | "saturday";

/** The choices of *Region* (CFG-04): where the person is, as far as times and numbers go. */
export interface Region {
  /** A time zone name like `Europe/Berlin`, or `device` for the one of this device. */
  readonly timeZone: string;
  readonly dateFormat: DateFormat;
  readonly numberFormat: NumberFormat;
  readonly firstDay: FirstDay;
}

/** Everything from the device: what a person who chose nothing sees. */
export const deviceRegion: Region = {
  timeZone: "device",
  dateFormat: "device",
  numberFormat: "device",
  firstDay: "device",
};

/**
 * Whether this browser knows the time zone. The server only checks that a name is shaped like one
 * (it has no list of zones), so a name this browser does not know is left out here: the times then
 * show in the zone of the device and the page does not break.
 */
export function isKnownTimeZone(name: string): boolean {
  try {
    return new Intl.DateTimeFormat("en", { timeZone: name }).resolvedOptions().timeZone !== "";
  } catch {
    return false;
  }
}

let current: Region = deviceRegion;
const listeners = new Set<() => void>();

/** The region the formats use now. */
export function currentRegion(): Region {
  return current;
}

/** Changes the region and tells everybody who shows times or numbers. */
export function setRegion(next: Region): void {
  const same =
    next.timeZone === current.timeZone &&
    next.dateFormat === current.dateFormat &&
    next.numberFormat === current.numberFormat &&
    next.firstDay === current.firstDay;
  if (same) {
    return;
  }
  current = next;
  for (const listener of listeners) {
    listener();
  }
}

/** The region, for a component that shows times or numbers: it shows them again when it changes. */
export function useRegion(): Region {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    () => current,
  );
}
