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

/**
 * The cents of an amount typed in the way the person writes numbers (`12,50` or `12.50`, with the
 * groups their format has), or `null` when what they typed is no amount with at most two places.
 * Spaces do not matter; the whole part has at most 12 digits, so the cents stay a safe whole number.
 * The one place that reads an amount of money.
 */
export function centsIn(text: string, region: Region = currentRegion()): number | null {
  const { decimal, group } = separatorsOf(region);
  const unspaced = text.replaceAll(/\s/gu, "");
  const ungrouped = /^\s*$/u.test(group) ? unspaced : unspaced.split(group).join("");
  const plain = decimal === "." ? ungrouped : ungrouped.replace(decimal, ".");
  const shape = /^(\d{1,12})(?:\.(\d{1,2}))?$/u;
  const found = shape.exec(plain);
  if (found === null) {
    return null;
  }
  const [, whole = "0", places = ""] = found;
  return Number(whole) * 100 + Number(places.padEnd(2, "0"));
}
