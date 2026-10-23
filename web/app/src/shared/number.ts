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
