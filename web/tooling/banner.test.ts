import { describe, expect, it } from "vitest";

import { colorsOf, looksOf, readMedia, tokensOf } from "./media-theme.ts";

// The banner of the front page (docs/media/banner.svg) is a picture, so it cannot read the theme
// file; its colors are copied from it, and this test keeps the copies the same as the tokens.

const banner = readMedia("banner.svg");
const { light, dark } = looksOf(banner);

describe("the banner of the front page", () => {
  it("uses the colors of the default light theme in its light look", () => {
    expect(colorsOf(light)).toEqual(tokensOf(light, "daylight"));
    expect(colorsOf(light)).toHaveLength(11);
  });

  it("uses the colors of the default dark theme in its dark look", () => {
    expect(colorsOf(dark)).toEqual(tokensOf(dark, "dusk"));
    expect(colorsOf(dark)).toHaveLength(11);
  });

  it("stands still when a person asks for less motion", () => {
    expect(banner).toMatch(/@media \(prefers-reduced-motion: reduce\)[^}]*\{[^}]*animation: none/u);
  });

  it("shoos a bug away and says boo, and the bug and the word are still when motion is reduced", () => {
    expect(banner).toContain('class="bug"');
    expect(banner).toContain(">Boo!<");
    // The tiles, the walk of the bug, the step to Done, the shoo and the boo all stand still.
    expect(banner).toMatch(
      /\.tiles > g, \.walk, \.legs, \.here, \.done \.fill, \.bug, \.boo \{ animation: none/u,
    );
  });

  it("plays one calm sequence once: nothing repeats, blinks or changes its color", () => {
    expect(banner).not.toMatch(/infinite|alternate/u);
    const frames = banner
      .split("@keyframes ")
      .slice(1)
      .map((piece) => piece.slice(0, piece.indexOf("}\n")));
    expect(frames.length).toBeGreaterThan(0);
    for (const frame of frames) {
      expect(frame).not.toMatch(/fill:|stroke:|color:/u);
    }
  });

  it("loads nothing from outside and runs no script", () => {
    expect(banner).not.toMatch(/<image|<script|href=|url\(|https?:\/\/(?!www\.w3\.org)/u);
  });
});

describe("the words of the banner", () => {
  it("shows what Bugaboo does, and says which parts are still planned", () => {
    for (const words of [
      "Issues and your own workflows",
      "Gantt charts",
      "AI agents",
      "Built for the GDPR",
    ]) {
      expect(banner).toContain(`>${words}<`);
    }
    expect(banner.match(/>Planned: /gu)).toHaveLength(2);
    expect(banner).toContain("Under development");
  });
});
