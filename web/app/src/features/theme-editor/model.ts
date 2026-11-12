import { colorTokens, type ColorToken, type OwnFont, type OwnRadius } from "@bugaboo/ui";
import type { ColorRating, Rating } from "@bugaboo/wasm";

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

/** A value as JSON text with the keys of every object in order, so the order they came in is no difference. */
function canonical(value: unknown): string {
  if (Array.isArray(value)) {
    return `[${value.map((item) => canonical(item)).join(",")}]`;
  }
  if (typeof value === "object" && value !== null) {
    const entries = Object.keys(value)
      .toSorted()
      .map((key) => `${JSON.stringify(key)}:${canonical(Reflect.get(value, key))}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value);
}

/** Whether two sets of values are the same, whatever the order of their keys. */
export function sameValues(one: ThemeValues, other: ThemeValues): boolean {
  return canonical(one) === canonical(other);
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

/**
 * The values with the color `token` set to `value`. Its color for more contrast stays; a color the
 * theme did not change yet starts with the built-in theme's color for more contrast (`startMore`,
 * the maintainer's decision on M8-F70), unless that is the color itself.
 */
export function withColor(
  values: ThemeValues,
  token: ColorToken,
  value: string,
  startMore?: string,
): ThemeValues {
  const own = values.colors?.[token];
  const more = own === undefined ? startMore : (own.more ?? undefined);
  const color =
    more === undefined || more.toLowerCase() === value.toLowerCase() ? { value } : { value, more };
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

/** The worst rating of a color as `bb-core` worked it out, as it is or with more contrast. */
export function ratingOf(
  ratings: readonly ColorRating[],
  token: ColorToken,
  moreContrast: boolean,
): Rating | undefined {
  const found = ratings.find((rating) => rating.name === token);
  return (moreContrast ? found?.more : found?.usual) ?? undefined;
}

/** The plain name of a color of a pair, or the token itself for one the editor has no name for. */
export function colorName(token: string): string {
  return isColorToken(token) ? colorWords[token].label : token;
}
