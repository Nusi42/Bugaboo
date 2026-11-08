import { currentRegion, numberLocales, type Region } from "./region.ts";

/** The least and the most a number setting may be. */
export interface Bounds {
  readonly least: number;
  readonly most: number;
}

/**
 * The whole number a person typed, or `null` when what they typed is none, has more than 15 digits
 * (a number of more digits loses its last digits as a JavaScript number, so it is never sent) or
 * lies outside the bounds. Spaces around it do not matter. The one place that reads a whole number.
 */
export function wholeNumberIn(text: string, bounds?: Bounds): number | null {
  const trimmed = text.trim();
  if (!/^[+-]?\d{1,15}$/u.test(trimmed)) {
    return null;
  }
  const value = Number(trimmed);
  if (bounds !== undefined && (value < bounds.least || value > bounds.most)) {
    return null;
  }
  return value;
}

/** The parts of a decimal number as the region writes it: the mark after the whole part, and what groups thousands. */
function separatorsOf(region: Region): { readonly decimal: string; readonly group: string } {
  const parts = new Intl.NumberFormat(numberLocales[region.numberFormat]).formatToParts(1234567.5);
  const valueOf = (type: Intl.NumberFormatPartTypes) =>
    parts.find((part: Readonly<Intl.NumberFormatPart>) => part.type === type)?.value ?? "";
  return { decimal: valueOf("decimal"), group: valueOf("group") };
}

/** The text as a pattern that matches it literally. */
function literal(text: string): string {
  return text.replaceAll(/[.*+?^${}()|[\]\\]/gu, String.raw`\$&`);
}

/**
 * The cents of an amount typed in the way the person writes numbers (`12,50` or `12.50`, with the
 * groups their format has), or `null` when what they typed is no amount with at most two places.
 * A group mark stands only between groups of three digits (`1,234.50`, never `1,2,3`); the whole part
 * has at most 12 digits, so the cents stay a safe whole number. The one place that reads an amount
 * of money.
 */
export function centsIn(text: string, region: Region = currentRegion()): number | null {
  const { decimal, group } = separatorsOf(region);
  const [whole = "", places, ...rest] = text.trim().split(decimal);
  const mark = /^\s*$/u.test(group) ? String.raw`\s` : literal(group);
  const grouped = new RegExp(`^[0-9]{1,3}(?:${mark}[0-9]{3})+$`, "u");
  const digits = grouped.test(whole) ? whole.replaceAll(new RegExp(mark, "gu"), "") : whole;
  const shaped = /^\d{1,12}$/u.test(digits) && (places === undefined || /^\d{1,2}$/u.test(places));
  if (!shaped || rest.length > 0) {
    return null;
  }
  return Number(digits) * 100 + Number((places ?? "").padEnd(2, "0"));
}
