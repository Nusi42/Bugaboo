import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { repoRoot } from "./ports.ts";

// The banner of the front page (docs/media/banner.svg) is a picture, so it cannot read the theme
// file; its colors are copied from it, and this test keeps the copies the same as the tokens.

const banner = readFileSync(join(repoRoot, "docs/media/banner.svg"), "utf8");
const theme = readFileSync(join(repoRoot, "design/theme.kdl"), "utf8");

/** The value of a token for one theme of `design/theme.kdl`, such as `#5b3fd9`. */
function tokenValue(token: string, themeName: string): string {
  const line = theme.split("\n").find((each) => each.trim().startsWith(`color "${token}" `));
  return new RegExp(` ${themeName}="(#[0-9a-f]{6})"`, "u").exec(line ?? "")?.[1] ?? "no such token";
}

/** The custom properties of a block of the banner, as `token: value` lines (`bg-canvas: #ffffff`). */
function colorsOf(block: string): string[] {
  return (block.match(/--[a-z-]+: #[0-9a-f]{6};/gu) ?? []).map((line) => line.slice(2, -1));
}

/** What the tokens of a theme say about the same names, as `token: value` lines in the same order. */
function tokensOf(block: string, themeName: string): string[] {
  return colorsOf(block).map((line) => {
    const token = line.slice(0, line.indexOf(":"));
    return `${token}: ${tokenValue(token, themeName)}`;
  });
}

const [light = "", afterLight = ""] = banner.split("@media (prefers-color-scheme: dark)");
const dark = afterLight.split("text {")[0] ?? "";

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

  it("loads nothing from outside and runs no script", () => {
    expect(banner).not.toMatch(/<image|<script|href=|url\(|https?:\/\/(?!www\.w3\.org)/u);
  });
});
