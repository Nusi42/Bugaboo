/**
 * Where a word stands in a text. The Rust code counts bytes of the UTF-8 text; the browser counts
 * the units of a string (UTF-16). This turns the one into the other, so a mistake or a suggestion
 * marks the same word in both.
 */
export interface Offsets {
  /** The place in the string for a place in bytes; both are the start or the end of a character. */
  readonly inString: (byte: number) => number;
  /** The place in bytes for a place in the string. */
  readonly inBytes: (unit: number) => number;
}

const encoder = new TextEncoder();

/** The places of a text. A text of single bytes (the usual one) needs no table. */
export function offsetsOf(text: string): Offsets {
  if (encoder.encode(text).length === text.length) {
    return { inString: (byte) => byte, inBytes: (unit) => unit };
  }
  const unitAtByte = new Map<number, number>();
  let byte = 0;
  let unit = 0;
  for (const character of text) {
    unitAtByte.set(byte, unit);
    byte += encoder.encode(character).length;
    unit += character.length;
  }
  unitAtByte.set(byte, unit);
  return {
    inString: (at) => unitAtByte.get(at) ?? text.length,
    // A place inside a pair of units is the place before the character.
    inBytes: (at) => {
      const before = text.slice(0, at);
      const half = /[\uD800-\uDBFF]$/u.test(before);
      return encoder.encode(half ? before.slice(0, -1) : before).length;
    },
  };
}

/** The keys under which a place is stored in the answers of the module. */
const PLACE_KEYS: ReadonlySet<string> = new Set(["span", "replace"]);

function isPlace(value: unknown): value is { start: number; end: number } {
  return (
    typeof value === "object" &&
    value !== null &&
    "start" in value &&
    typeof value.start === "number" &&
    "end" in value &&
    typeof value.end === "number"
  );
}

/** The answer of the module with every place (`span`, `replace`) in units of the string. */
export function inStringUnits(value: unknown, offsets: Offsets): unknown {
  if (Array.isArray(value)) {
    return value.map((item) => inStringUnits(item, offsets));
  }
  if (typeof value !== "object" || value === null) {
    return value;
  }
  const converted: Record<string, unknown> = {};
  for (const [key, member] of Object.entries(value)) {
    converted[key] =
      PLACE_KEYS.has(key) && isPlace(member)
        ? { start: offsets.inString(member.start), end: offsets.inString(member.end) }
        : inStringUnits(member, offsets);
  }
  return converted;
}
