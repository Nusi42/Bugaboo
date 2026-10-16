/**
 * Values of own fields as people write them and as the API words them (CF-01): a length of time
 * is seconds in the API and "1h 30m" on the screen; a decimal is text in both.
 */

import { decimalDigits, decimalParts } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { durationText, SECONDS_PER_HOUR, SECONDS_PER_MINUTE } from "../../shared/own-value-text.ts";
import type { OwnField } from "./fields.ts";

/** A unit as it is written in a pattern. */
function escaped(unit: string): string {
  return unit.replaceAll(/[.*+?^${}()|[\]\\]/gu, String.raw`\$&`);
}

/** The pattern of a length of time typed with the units of the person's language. */
function durationPattern(): RegExp {
  const [hour, minute] = [escaped(t("fields.unit-hour")), escaped(t("fields.unit-minute"))];
  return new RegExp(String.raw`^\s*(?:(\d+)\s*${hour})?\s*(?:(\d+)\s*${minute})?\s*$`, "iu");
}

/**
 * The seconds of a length of time typed as "1h 30m", "90m" or "2h"; a plain number is minutes.
 * `undefined` for text that is none, so the screen can say so.
 */
export function durationSeconds(text: string): number | undefined {
  const trimmed = text.trim();
  if (/^\d+$/u.test(trimmed)) {
    return Number(trimmed) * SECONDS_PER_MINUTE;
  }
  const found = durationPattern().exec(trimmed);
  if (trimmed === "" || found === null) {
    return undefined;
  }
  const [, hours, minutes] = found;
  return Number(hours ?? 0) * SECONDS_PER_HOUR + Number(minutes ?? 0) * SECONDS_PER_MINUTE;
}

/** Whether the text is a decimal number as the API takes it: digits, an optional sign and point. */
export function isDecimalText(text: string): boolean {
  return new RegExp(String.raw`^[-+]?\d+(?:\.\d{1,${decimalDigits}})?$`, "u").test(text.trim());
}

/** Whether the text is a whole number as the API takes it. */
export function isWholeNumberText(text: string): boolean {
  return /^[-+]?\d+$/u.test(text.trim());
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
