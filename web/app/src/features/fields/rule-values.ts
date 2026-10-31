/**
 * The values of the boxes of a field's *Rules* (CF-03): the lowest and highest value as people
 * write them for the kind of the field, and as the API stores them.
 */

import type { LengthReader } from "../../shared/length.ts";
import { wholeNumberIn } from "../../shared/number.ts";
import { durationText } from "../../shared/own-value-text.ts";
import type { OwnField } from "./fields.ts";
import {
  boundDay,
  dayBound,
  type Bound,
  decimalToMillionths,
  durationStored,
  millionthsToDecimal,
} from "./value-text.ts";

/** Whether the bounds of the field are days (a *Date* or a *Date and time*). */
function isDated(field: OwnField): boolean {
  return field.kind === "date" || field.kind === "date-time";
}

/** The lowest or highest value in a box, as people write it for this kind. */
export function boxOf(field: OwnField, stored: number | null | undefined): string {
  if (stored === undefined || stored === null) {
    return "";
  }
  if (isDated(field)) {
    return boundDay(field.kind, stored);
  }
  if (field.kind === "decimal") {
    return millionthsToDecimal(stored);
  }
  return field.kind === "duration" ? durationText(stored) : String(stored);
}

/** What a box means in the unit the API stores; `null` for an empty box, `undefined` for text that is none. */
export function storedOf(
  field: OwnField,
  text: string,
  end: Bound,
  readLength: LengthReader | undefined,
): number | null | undefined {
  const trimmed = text.trim();
  if (trimmed === "") {
    return null;
  }
  if (isDated(field)) {
    return dayBound(field.kind, trimmed, end);
  }
  if (field.kind === "decimal") {
    return decimalToMillionths(trimmed);
  }
  if (field.kind === "duration") {
    return durationStored(trimmed, readLength);
  }
  return wholeNumberIn(trimmed) ?? undefined;
}

/** A count of characters or values in a box: a whole number, or nothing. */
export function countOf(text: string): number | null | undefined {
  const trimmed = text.trim();
  if (trimmed === "") {
    return null;
  }
  return /^\d+$/u.test(trimmed) ? Number(trimmed) : undefined;
}
