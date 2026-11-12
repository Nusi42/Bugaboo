import { readFileSync } from "node:fs";

import { themeIds } from "@bugaboo/ui";
import { moduleLocation, startThemeReader, type ThemeRead } from "@bugaboo/wasm";
import { describe, expect, it } from "vitest";

import {
  ratingOf,
  sameValues,
  withColor,
  withMore,
  withoutColor,
  type ThemeValues,
} from "../app/src/features/theme-editor/index.ts";

/**
 * The editor rates a theme with the module of `bb-core`, the code the server rates a save with
 * (ADR-0026, update M8). These are the cases of `bb-core`'s own tests (`theme/tests.rs`), asked
 * through the editor's reader, so its answers are the server's.
 */
const reader = startThemeReader(() => readFileSync(moduleLocation));

/** The rating of a theme the reader takes; a theme it refuses fails the test that asked. */
function rated(theme: string, values: ThemeValues): ThemeRead {
  const found = reader(theme, values);
  if (found === null) {
    throw new Error(`the reader refused ${theme} ${JSON.stringify(values)}`);
  }
  return found;
}

/** The color a token shows in a rated theme. */
function shownIn(found: ThemeRead, token: string): string | undefined {
  return found.colors.find((color) => color.name === token)?.value;
}

describe("thm_03_the_editor_rates_each_pair_like_the_server", () => {
  it.each(themeIds)("finds nothing hard to read in %s as it is", (theme) => {
    const found = rated(theme, {});
    expect(found.summary.hard_to_read).toBe(0);
    expect(found.hard_colors).toEqual([]);
    expect(found.summary.easy).toBeGreaterThan(0);
  });

  it("names the pairs hard to read, as many as the sums say, and the colors to change", () => {
    const found = rated("daylight", withColor({}, "fg", "#fafafa"));
    expect(found.hard).toContainEqual(
      expect.objectContaining({ first: "fg", second: "bg-canvas" }),
    );
    expect(found.hard).toHaveLength(found.summary.hard_to_read);
    expect(found.hard_colors).toHaveLength(found.summary.colors_hard_to_read);
    expect(found.hard_colors.map((color) => color.place)).toContain("fg");
    expect(ratingOf(found.ratings, "fg", false)).toBe("hard-to-read");
  });

  it("follows the links of the built-in theme for what the theme does not change", () => {
    const found = rated("daylight", withColor({}, "border-control", "#112233"));
    expect(shownIn(found, "border-control")).toBe("#112233");
    expect(shownIn(found, "hover-line")).toBe("#112233");
    expect(shownIn(found, "fg")).toBe(shownIn(rated("daylight", {}), "fg"));
  });

  it.each([
    { colors: { "no-such-token": { value: "#112233" } } },
    { colors: { fg: { value: "red" } } },
    { colors: { fg: { value: "#fff" } } },
    { colors: { fg: { value: "#112233", more: "#112233" } } },
    { radii: { "r-ctl": 400 } },
    { radii: { "r-pill": 4 } },
    { fonts: { "font-ui": "Comic Sans" } },
    { fonts: { "font-mono": "font-ui" } },
    { shadow: [{ x: 0, y: 1, blur: 2, spread: 0, color: "red" }] },
  ] as const)("refuses values a save would refuse: %j", (values) => {
    expect(reader("daylight", values)).toBeNull();
  });

  it("knows no built-in theme that is not there", () => {
    expect(reader("nowhere", {})).toBeNull();
  });
});

describe("thm_03_the_editor_rates_the_color_for_more_contrast_like_the_server", () => {
  it("rates the color for more contrast apart from the color itself", () => {
    const usual = withColor({}, "fg-muted", "#6b7280");
    const mended = withMore(usual, "fg-muted", "#3f4652", "#6b7280");
    expect(ratingOf(rated("daylight", usual).ratings, "fg-muted", true)).toBe("hard-to-read");
    expect(ratingOf(rated("daylight", mended).ratings, "fg-muted", true)).not.toBe("hard-to-read");
    expect(rated("daylight", withoutColor(usual, "fg-muted")).summary.hard_to_read).toBe(0);
  });
});

describe("thm_03_the_editor_keeps_values_as_a_save_takes_them", () => {
  it("starts the color for more contrast from the built-in theme's (M8-F70)", () => {
    const changed = withColor({}, "fg-muted", "#4f5563", "#464c57");
    expect(changed.colors?.["fg-muted"]).toEqual({ value: "#4f5563", more: "#464c57" });
    // The color itself is never its own color for more contrast.
    expect(withColor({}, "fg-muted", "#464c57", "#464c57").colors?.["fg-muted"]).toEqual({
      value: "#464c57",
    });
  });

  it("finds two values the same whatever the order of their keys", () => {
    const one = { colors: { fg: { value: "#111111" } }, radii: { "r-ctl": 4 } };
    const other = { radii: { "r-ctl": 4 }, colors: { fg: { value: "#111111" } } };
    expect(sameValues(one, other)).toBe(true);
    expect(sameValues(one, { radii: { "r-ctl": 5 } })).toBe(false);
  });
});
