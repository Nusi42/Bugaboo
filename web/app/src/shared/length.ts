/**
 * A length of time a person types, read by the one reader of `bb-core` (`time::length`), which the
 * browser runs through `bb-wasm` (ADR-0035): `1:30`, `1.5` hours with the person's decimal mark,
 * `90m`, `1h 30m`. The module comes with the shared wording (`use-wording.ts`), so a screen that
 * reads a length loads it when it opens.
 */

import { t } from "@bugaboo/messages";
import type { LengthRead, LengthTyped, Wording } from "@bugaboo/wasm";

import { deviceDecimalFormat } from "./format.ts";
import { currentRegion, type Region } from "./region.ts";

/** Reads a typed length: the minutes, or the sentence of the mistake. */
export type LengthReader = (text: string) => LengthRead;

/**
 * A typed length as the reader takes it: with the *Number format* whose decimal mark `1,5` is read
 * with (the device's named by the fixed one that writes its mark) and the short words of the units
 * of the catalog. With the minutes of a work day of the person a length is for (`dayMinutes`), days
 * count too, in that person's own work hours (*Takes me 10 days*, PLAN-14).
 */
export function lengthTyped(
  text: string,
  region: Region = currentRegion(),
  dayMinutes?: number,
): LengthTyped {
  return {
    text,
    number_format: region.numberFormat === "device" ? deviceDecimalFormat() : region.numberFormat,
    hour_unit: t("fields.unit-hour"),
    minute_unit: t("fields.unit-minute"),
    ...(dayMinutes === undefined
      ? {}
      : { day_units: t("fields.unit-day-words"), day_minutes: dayMinutes }),
  };
}

/**
 * The reader of a length without a bound (an own field), with the person's region of now; with
 * `dayMinutes`, days count in the work hours of the person the length is for.
 */
export function lengthReaderOf(wording: Wording, dayMinutes?: number): LengthReader {
  return (text) => wording.readLength(lengthTyped(text, currentRegion(), dayMinutes));
}
