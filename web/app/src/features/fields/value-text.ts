/**
 * Values of own fields as people write them and as the API words them (CF-01): a length of time
 * is seconds in the API and "1h 30m" on the screen; a decimal is text in both.
 */

import {
  decimalDigits,
  decimalParts,
  decimalWholeDigits,
  msPerDay,
  secondsPerHour,
  secondsPerMinute,
} from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { dayNumbersOf, isCalendarDay } from "../../shared/format.ts";
import { wholeNumberIn } from "../../shared/number.ts";
import { durationText } from "../../shared/own-value-text.ts";
import type { OwnField, Sent } from "./fields.ts";

/**
 * The most digits of a number a person types in a length of time. With nine digits the seconds
 * (nine digits of hours) stay far under the largest whole number a JavaScript number holds, so
 * nothing is rounded on its way. A decimal has its own limit, the server's (`decimalWholeDigits`).
 */
const MOST_DIGITS = 9;

/** A unit as it is written in a pattern. */
function escaped(unit: string): string {
  return unit.replaceAll(/[.*+?^${}()|[\]\\]/gu, String.raw`\$&`);
}

/** The pattern of a length of time typed with the units of the person's language. */
function durationPattern(): RegExp {
  const [hour, minute] = [escaped(t("fields.unit-hour")), escaped(t("fields.unit-minute"))];
  const digits = String.raw`(\d{1,${MOST_DIGITS}})`;
  return new RegExp(
    String.raw`^\s*(?:${digits}\s*${hour})?\s*(?:${digits}\s*${minute})?\s*$`,
    "iu",
  );
}

/**
 * The seconds of a length of time typed as "1h 30m", "90m" or "2h"; a plain number is minutes.
 * `undefined` for text that is none, so the screen can say so.
 */
export function durationSeconds(text: string): number | undefined {
  const trimmed = text.trim();
  if (new RegExp(String.raw`^\d{1,${MOST_DIGITS}}$`, "u").test(trimmed)) {
    return Number(trimmed) * secondsPerMinute;
  }
  const found = durationPattern().exec(trimmed);
  if (trimmed === "" || found === null) {
    return undefined;
  }
  const [, hours, minutes] = found;
  return Number(hours ?? 0) * secondsPerHour + Number(minutes ?? 0) * secondsPerMinute;
}

/** Whether the text is a decimal number as the API takes it: digits, an optional sign and point. */
export function isDecimalText(text: string): boolean {
  return new RegExp(
    String.raw`^[-+]?\d{1,${decimalWholeDigits}}(?:\.\d{1,${decimalDigits}})?$`,
    "u",
  ).test(text.trim());
}

/** What the text of a box means for the API, or a sentence that says why it cannot be sent. */
export function sentFrom(
  field: OwnField,
  text: string,
): { readonly value: Sent } | { readonly problem: string } {
  const trimmed = text.trim();
  if (trimmed === "") {
    return { value: null };
  }
  if (field.kind === "whole-number") {
    const whole = wholeNumberIn(trimmed);
    return whole === null ? { problem: t("fields.not-a-whole-number") } : { value: whole };
  }
  if (field.kind === "decimal") {
    return isDecimalText(trimmed) ? { value: trimmed } : { problem: t("fields.not-a-decimal") };
  }
  if (field.kind === "duration") {
    const seconds = durationSeconds(trimmed);
    return seconds === undefined ? { problem: t("fields.not-a-duration") } : { value: seconds };
  }
  return { value: trimmed };
}

/** The non-empty lines of a text, trimmed: one choice on each line. */
export function wordsOfLines(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

const MILLIONTHS = decimalParts;

/** A decimal typed as "12.5" in millionths, the way the API stores it; `undefined` when it is none. */
export function decimalToMillionths(text: string): number | undefined {
  const trimmed = text.trim();
  if (!isDecimalText(trimmed)) {
    return undefined;
  }
  const negative = trimmed.startsWith("-");
  const digits = trimmed.replace(/^[-+]/u, "");
  const [whole = "0", parts = ""] = digits.split(".");
  const amount = Number(whole) * MILLIONTHS + Number(parts.padEnd(decimalDigits, "0"));
  // A decimal the server takes can have more millionths than a JavaScript number holds exactly.
  if (!Number.isSafeInteger(amount)) {
    return undefined;
  }
  return negative ? -amount : amount;
}

/** Millionths as a decimal without trailing zeros: 12500000 is "12.5". */
export function millionthsToDecimal(millionths: number): string {
  const sign = millionths < 0 ? "-" : "";
  const amount = Math.abs(millionths);
  const whole = Math.floor(amount / MILLIONTHS);
  const fraction = String(amount % MILLIONTHS)
    .padStart(decimalDigits, "0")
    .replace(/0+$/u, "");
  return fraction === "" ? `${sign}${whole}` : `${sign}${whole}.${fraction}`;
}

/** The text a value has in a box: what a person edits. */
export function textOf(field: OwnField, value: unknown): string {
  if (value === undefined || value === null) {
    return "";
  }
  if (field.kind === "duration" && typeof value === "number") {
    return durationText(value);
  }
  return typeof value === "string" || typeof value === "number" ? String(value) : "";
}

/**
 * What a box in a row of the Details last sent, and the value it was sent against. The value the
 * page shows is read again after a save, and until that read comes back it is still the old one: a
 * box that is left in between must not send what it has sent already.
 */
export interface Kept {
  /** The value the page showed when the text was sent. */
  readonly from: string;
  /** The text that was sent. */
  readonly text: string;
}

/** The text the box counts as saved: the last text sent while the value is still the one it was sent against. */
export function savedText(shown: string, kept?: Kept): string {
  return kept !== undefined && kept.from === shown ? kept.text : shown;
}

/** Whether leaving the box sends nothing: it has what is saved already. */
export function isUnchanged(typed: string, shown: string, kept?: Kept): boolean {
  return typed === savedText(shown, kept);
}

/** How many of the stored unit make a day: a *Date* is stored in days, a *Date and time* in microseconds. */
function partsPerDay(kind: OwnField["kind"]): number {
  return kind === "date" ? 1 : msPerDay * 1000;
}

/** Which end of a range a bound is: the lowest value allowed, or the highest. */
export const bound = { lowest: "lowest", highest: "highest" } as const;
export type Bound = (typeof bound)[keyof typeof bound];

/**
 * The bound of a range in the unit the API stores, for a day typed as `2026-10-25`; `undefined`
 * for text that is no day. The lowest bound is the start of that day (UTC). The highest bound of a
 * *Date and time* is the last moment of that day, so the day itself is still allowed; a *Date* is
 * a whole day already.
 */
export function dayBound(kind: OwnField["kind"], day: string, end: Bound): number | undefined {
  if (!isCalendarDay(day)) {
    return undefined;
  }
  const [year, month, date] = dayNumbersOf(day);
  const start = (Date.UTC(year, month - 1, date) / msPerDay) * partsPerDay(kind);
  return end === bound.highest && kind !== "date" ? start + partsPerDay(kind) - 1 : start;
}

/** The day of a stored bound, as `2026-10-25`. */
export function boundDay(kind: OwnField["kind"], stored: number): string {
  return new Date((stored / partsPerDay(kind)) * msPerDay).toISOString().slice(0, 10);
}
