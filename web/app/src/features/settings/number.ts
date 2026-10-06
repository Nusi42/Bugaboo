/** The least and the most a number setting may be. */
export interface Bounds {
  readonly least: number;
  readonly most: number;
}

/**
 * The whole number a person typed, or `null` when what they typed is none or lies outside the
 * bounds (so it is never sent). Spaces around it do not matter.
 */
export function numberIn(text: string, bounds?: Bounds): number | null {
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
