import { describe, expect, it } from "vitest";

import { lengthInPixels } from "./css-length.ts";

const properties: Readonly<Record<string, string>> = {
  "--text-scale": "1.25",
  "--density": "0.75",
  "--size-col-key": "calc(80px * var(--text-scale))",
  "--space-3": "calc(12px * var(--density))",
  "--pad-box": "var(--space-3)",
  "--size-control": "44px",
  "--loop": "var(--loop)",
};
const read = (name: string): string => properties[name] ?? "";

describe("ux_7_4 the length a token is, in pixels", () => {
  it("reads a plain length", () => {
    expect(lengthInPixels("var(--size-control)", read)).toBe(44);
    expect(lengthInPixels("2rem", read)).toBe(32);
  });

  it("counts the size of the text in a calc, and not only the first number", () => {
    expect(lengthInPixels("var(--size-col-key)", read)).toBe(100);
  });

  it("counts the density, and follows a token that is another token", () => {
    expect(lengthInPixels("var(--pad-box)", read)).toBe(9);
  });

  it("adds, subtracts and takes brackets", () => {
    expect(lengthInPixels("calc(var(--size-control) + (8px - 2px) * 2)", read)).toBe(56);
  });

  it("counts what it does not know as nothing, and ends a loop", () => {
    expect(lengthInPixels("70dvh", read)).toBe(0);
    expect(lengthInPixels("var(--nothing)", read)).toBe(0);
    expect(lengthInPixels("var(--loop)", read)).toBe(0);
  });
});
