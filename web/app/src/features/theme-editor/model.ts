import { colorTokens, type ColorToken, type OwnFont, type OwnRadius } from "@bugaboo/ui";
import type { RatedPair, Rating } from "@bugaboo/wasm";

import { colorWords, type ColorGroup } from "./color-words.ts";
import type { ShadowLayer, ThemeValues } from "./themes.ts";

/** Whether a token is a color an own theme may set: one of the generated list. */
export function isColorToken(name: string): name is ColorToken {
  return colorTokens.some((token) => token === name);
}

/** A whole number as typed, or `null` for anything else. */
export function wholeNumber(text: string): number | null {
  const trimmed = text.trim();
  return /^-?\d+$/u.test(trimmed) ? Number(trimmed) : null;
}

/** Whether two sets of values are the same. */
export function sameValues(one: ThemeValues, other: ThemeValues): boolean {
  return JSON.stringify(one) === JSON.stringify(other);
}

/** A map of values without `key`; none when nothing is left. */
function withoutKey<Value extends string | number | object>(
  map: Readonly<Record<string, Value>> | undefined,
  key: string,
): Readonly<Record<string, Value>> | undefined {
  const rest: Record<string, Value> = {};
  for (const name of Object.keys(map ?? {})) {
    const value = map?.[name];
    if (name !== key && value !== undefined) {
      rest[name] = value;
    }
  }
  return Object.keys(rest).length === 0 ? undefined : rest;
}

/** The values without what `part` sets: the built-in theme's again. */
function withoutPart(values: ThemeValues, part: keyof ThemeValues): ThemeValues {
  const next = { ...values };
  delete next[part];
  return next;
}

/** The values with the map `part` set to `rest`, or without it when nothing is left in it. */
function withMap(
  values: ThemeValues,
  part: "colors" | "fonts",
  rest: Readonly<Record<string, string | object>> | undefined,
): ThemeValues {
  return rest === undefined ? withoutPart(values, part) : { ...values, [part]: rest };
}

/** The values with the color `token` set to `value`; its color for more contrast stays. */
export function withColor(values: ThemeValues, token: ColorToken, value: string): ThemeValues {
  const more = values.colors?.[token]?.more;
  const color = more === undefined || more === null ? { value } : { value, more };
  return { ...values, colors: { ...values.colors, [token]: color } };
}

/**
 * The values with the color of `token` for more contrast set to `more`, or taken away (`null`) so
 * the color itself shows there too. A token the theme does not change keeps `shown` as its color.
 */
export function withMore(
  values: ThemeValues,
  token: ColorToken,
  more: string | null,
  shown: string,
): ThemeValues {
  const value = values.colors?.[token]?.value ?? shown;
  return {
    ...values,
    colors: { ...values.colors, [token]: more === null ? { value } : { value, more } },
  };
}

/** The values with the color `token` of the built-in theme again (*Use Dusk's color*). */
export function withoutColor(values: ThemeValues, token: ColorToken): ThemeValues {
  return withMap(values, "colors", withoutKey(values.colors, token));
}

/** The values with a corner set in pixels. */
export function withRadius(values: ThemeValues, name: OwnRadius, pixels: number): ThemeValues {
  return { ...values, radii: { ...values.radii, [name]: pixels } };
}

/** The values with a text face set to the stack of `stack`; its own stack is the base theme's face. */
export function withFont(values: ThemeValues, name: OwnFont, stack: OwnFont): ThemeValues {
  return stack === name
    ? withMap(values, "fonts", withoutKey(values.fonts, name))
    : { ...values, fonts: { ...values.fonts, [name]: stack } };
}

/** The values with a shadow of these layers, or the built-in theme's shadow again (`null`). */
export function withShadow(
  values: ThemeValues,
  layers: readonly ShadowLayer[] | null,
): ThemeValues {
  return layers === null ? withoutPart(values, "shadow") : { ...values, shadow: layers };
}

/** The colors of a group that the theme changes, in the order of the theme file. */
export function changedIn(values: ThemeValues, group: ColorGroup): readonly ColorToken[] {
  return colorTokens.filter(
    (token) => colorWords[token].group === group && values.colors?.[token] !== undefined,
  );
}

/** The colors of a group, in the order of the theme file. */
export function colorsIn(group: ColorGroup): readonly ColorToken[] {
  return colorTokens.filter((token) => colorWords[token].group === group);
}

/** How bad a rating is, so the worst of several can be told. */
const badness: Readonly<Record<Rating, number>> = { easy: 0, ok: 1, "hard-to-read": 2 };

/**
 * The worst rating of the pairs a color is in, as it is or with more contrast; none when the color
 * is in no pair. The ratings are `bb-core`'s: this only picks the pairs of one color.
 */
export function ratingOf(
  pairs: readonly RatedPair[],
  token: ColorToken,
  moreContrast: boolean,
): Rating | undefined {
  let worst: Rating | undefined;
  for (const pair of pairs) {
    const inPair = pair.first === token || pair.second === token || pair.place === token;
    const worse = worst === undefined || badness[pair.rating] > badness[worst];
    if (inPair && pair.more_contrast === moreContrast && worse) {
      worst = pair.rating;
    }
  }
  return worst;
}

/** A pair that is hard to read, by its colors: what the message names and *Show* goes to. */
export interface NamedPair {
  readonly first: string;
  readonly second: string;
  /** The color to change to mend it. */
  readonly place: string;
  readonly more_contrast: boolean;
}

/** What tells a pair apart: its two colors and whether it is read with more contrast. */
export function pairKey(pair: NamedPair): string {
  return [pair.first, pair.second, String(pair.more_contrast)].join(" ");
}

/** The pairs that are hard to read, each once, in the order `bb-core` rated them. */
export function hardPairsOf(pairs: readonly RatedPair[]): readonly NamedPair[] {
  const hard = new Map<string, NamedPair>();
  for (const pair of pairs) {
    if (pair.rating === "hard-to-read" && !hard.has(pairKey(pair))) {
      hard.set(pairKey(pair), pair);
    }
  }
  return [...hard.values()];
}

/** The plain name of a color of a pair, or the token itself for one the editor has no name for. */
export function colorName(token: string): string {
  return isColorToken(token) ? colorWords[token].label : token;
}

/** A color that is hard to read with others: the color to change, and with how many it is hard to read. */
export interface HardColor {
  readonly place: string;
  readonly count: number;
}

/** The pairs that are hard to read, by the color to change to mend them, in the order `bb-core` rated them. */
export function hardColorsOf(pairs: readonly NamedPair[]): readonly HardColor[] {
  const counts = new Map<string, number>();
  for (const pair of pairs) {
    counts.set(pair.place, (counts.get(pair.place) ?? 0) + 1);
  }
  const colors: HardColor[] = [];
  for (const place of counts.keys()) {
    colors.push({ place, count: counts.get(place) ?? 0 });
  }
  return colors;
}
