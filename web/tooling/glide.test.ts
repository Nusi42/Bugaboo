import { describe, expect, it } from "vitest";

import { eased, glide } from "../app/e2e/support/glide.ts";

describe("eased", () => {
  it("starts at 0, ends at 1 and is slow at both ends", () => {
    expect(eased(0)).toBe(0);
    expect(eased(1)).toBe(1);
    expect(eased(0.5)).toBe(0.5);
    expect(eased(0.1)).toBeLessThan(0.1);
    expect(eased(0.9)).toBeGreaterThan(0.9);
  });
});

describe("glide", () => {
  it("ends at the place and leaves the start out", () => {
    const way = glide({ x: 0, y: 0 }, { x: 100, y: 50 }, 4);
    expect(way).toHaveLength(4);
    expect(way.at(-1)).toEqual({ x: 100, y: 50 });
    expect(way[0]?.x).toBeGreaterThan(0);
  });

  it("goes in a straight line, always forward", () => {
    const way = glide({ x: 10, y: 20 }, { x: 110, y: 70 });
    for (const point of way) {
      expect(point.y - 20).toBeCloseTo((point.x - 10) / 2);
    }
    const across = way.map((point) => point.x);
    expect(across).toEqual([...across].toSorted((one, two) => one - two));
    expect(new Set(across).size).toBe(across.length);
  });
});
