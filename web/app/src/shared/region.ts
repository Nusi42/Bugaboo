import {
  settingChoices,
  settingDefaults,
  settingKeys,
  type SettingChoice,
} from "@bugaboo/api-client";
import { setNumbersLocale } from "@bugaboo/messages";
import { useSyncExternalStore } from "react";

import type { MySettings } from "./my-settings.ts";
import { createStore } from "./store.ts";
import { oneOf, textOf } from "./setting-values.ts";

/** How dates are written, as the setting says it. */
export type DateFormat = SettingChoice<"regionDateFormat">;
/** On which clock a time of day is written, as the setting says it. */
export type TimeOfDay = SettingChoice<"regionTimeOfDay">;
/** How big numbers and decimals are written, as the setting says it. */
export type NumberFormat = SettingChoice<"regionNumberFormat">;
/** The day a week starts on, as the setting says it. */
export type FirstDay = SettingChoice<"regionFirstDay">;

/** The choices of *Region* (CFG-04): where the person is, as far as times and numbers go. */
export interface Region {
  /** A time zone name like `Europe/Berlin`, or `device` for the one of this device. */
  readonly timeZone: string;
  readonly dateFormat: DateFormat;
  /** `device` follows the date format: the device's clock with the device's date format, 24 hours with a fixed one. */
  readonly timeOfDay: TimeOfDay;
  readonly numberFormat: NumberFormat;
  readonly firstDay: FirstDay;
}

/** Everything from the device: what a person who chose nothing sees. */
export const deviceRegion: Region = {
  timeZone: settingDefaults.regionTimeZone,
  dateFormat: settingDefaults.regionDateFormat,
  timeOfDay: settingDefaults.regionTimeOfDay,
  numberFormat: settingDefaults.regionNumberFormat,
  firstDay: settingDefaults.regionFirstDay,
};

/**
 * Whether this browser knows the time zone. The server takes only the zones of its list
 * (`timeZones`), but a browser may be older than that list, so a name this browser does not know is
 * left out here: the times then show in the zone of the device and the page does not break.
 */
export function isKnownTimeZone(name: string): boolean {
  try {
    return new Intl.DateTimeFormat("en", { timeZone: name }).resolvedOptions().timeZone !== "";
  } catch (error) {
    // `Intl` refuses a name it does not know with a RangeError; any other failure is a real one.
    if (error instanceof RangeError) {
      return false;
    }
    throw error;
  }
}

/** The time zone of this device, by its name. */
export function deviceTimeZone(): string {
  return new Intl.DateTimeFormat().resolvedOptions().timeZone;
}

/** The region the settings describe. */
export function regionOf(settings?: MySettings): Region {
  const zone = textOf(settings, settingKeys.regionTimeZone, settingDefaults.regionTimeZone);
  return {
    timeZone:
      zone === deviceRegion.timeZone || isKnownTimeZone(zone) ? zone : deviceRegion.timeZone,
    dateFormat: oneOf(
      settingChoices.regionDateFormat,
      textOf(settings, settingKeys.regionDateFormat, settingDefaults.regionDateFormat),
      settingDefaults.regionDateFormat,
    ),
    timeOfDay: oneOf(
      settingChoices.regionTimeOfDay,
      textOf(settings, settingKeys.regionTimeOfDay, settingDefaults.regionTimeOfDay),
      settingDefaults.regionTimeOfDay,
    ),
    numberFormat: oneOf(
      settingChoices.regionNumberFormat,
      textOf(settings, settingKeys.regionNumberFormat, settingDefaults.regionNumberFormat),
      settingDefaults.regionNumberFormat,
    ),
    firstDay: oneOf(
      settingChoices.regionFirstDay,
      textOf(settings, settingKeys.regionFirstDay, settingDefaults.regionFirstDay),
      settingDefaults.regionFirstDay,
    ),
  };
}

/** The language tag that writes numbers the way each choice does; the device uses its own. */
export const numberLocales: Readonly<Record<NumberFormat, string | undefined>> = {
  device: undefined,
  point: "en-US",
  comma: "de-DE",
  space: "fr-FR",
};

const current = createStore<Region>(deviceRegion);

/** The region the formats use now. */
export function currentRegion(): Region {
  return current.snapshot();
}

/** Changes the region and tells everybody who shows times or numbers. */
export function setRegion(next: Region): void {
  const now = current.snapshot();
  const same =
    next.timeZone === now.timeZone &&
    next.dateFormat === now.dateFormat &&
    next.timeOfDay === now.timeOfDay &&
    next.numberFormat === now.numberFormat &&
    next.firstDay === now.firstDay;
  if (!same) {
    // The numbers in the texts first, so every screen that shows the new region writes them alike.
    setNumbersLocale(numberLocales[next.numberFormat]);
    current.replace(next);
  }
}

/** The region, for a component that shows times or numbers: it shows them again when it changes. */
export function useRegion(): Region {
  return useSyncExternalStore(current.subscribe, current.snapshot);
}
