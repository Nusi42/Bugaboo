/** How a custom property is read: its written value, or nothing where it has none. */
export type ReadProperty = (name: string) => string;

/** Pixels in a rem, where the page has not said otherwise. */
const REM = 16;

/** The most `var()` inside `var()` that is followed, so a loop in the theme ends. */
const DEEPEST = 8;

/** What is left of a text that is being read, and what was read from it. */
interface Read {
  readonly value: number;
  readonly rest: string;
}

/**
 * The length a token is, in pixels, as the page has it: the written value of a custom property is
 * `calc(80px * var(--text-scale))` and the like, and its first number is not its length. Reads
 * numbers with `px` or `rem`, `var()`, `calc()` and brackets, and `+ - * /`; what it does not know
 * (`dvh`, a function) counts as 0.
 */
export function lengthInPixels(written: string, read: ReadProperty, depth = 0): number {
  if (depth > DEEPEST) {
    return 0;
  }
  const sum = (text: string): Read => {
    let { value, rest } = product(text);
    for (;;) {
      const sign = /^\s*([+-])\s*/u.exec(rest)?.[1];
      if (sign === undefined) {
        return { value, rest };
      }
      const next = product(rest.replace(/^\s*[+-]\s*/u, ""));
      value = sign === "+" ? value + next.value : value - next.value;
      rest = next.rest;
    }
  };
  const product = (text: string): Read => {
    let { value, rest } = factor(text);
    for (;;) {
      const sign = /^\s*([*/])\s*/u.exec(rest)?.[1];
      if (sign === undefined) {
        return { value, rest };
      }
      const next = factor(rest.replace(/^\s*[*/]\s*/u, ""));
      value = sign === "*" ? value * next.value : value / (next.value === 0 ? 1 : next.value);
      rest = next.rest;
    }
  };
  const factor = (text: string): Read => {
    const trimmed = text.trimStart();
    const number = /^(-?[\d.]+)([a-z%]*)/u.exec(trimmed);
    if (number !== null) {
      // A number with no unit is a plain number (the scale of the text); an unknown unit is nothing.
      const unit = { "": 1, px: 1, rem: REM }[number[2] ?? ""] ?? 0;
      return { value: Number(number[1]) * unit, rest: trimmed.slice(number[0].length) };
    }
    const variable = /^var\(\s*(--[\w-]+)\s*(?:,[^)]*)?\)/u.exec(trimmed);
    if (variable !== null) {
      const inside = lengthInPixels(read(variable[1] ?? ""), read, depth + 1);
      return { value: inside, rest: trimmed.slice(variable[0].length) };
    }
    const bracket = /^(?:calc)?\(/u.exec(trimmed);
    if (bracket !== null) {
      const inner = sum(trimmed.slice(bracket[0].length));
      return { value: inner.value, rest: inner.rest.replace(/^\s*\)/u, "") };
    }
    return { value: 0, rest: trimmed.replace(/^[^\s)*/+-]+/u, "") };
  };
  return sum(written).value;
}
