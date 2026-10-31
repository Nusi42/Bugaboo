import { readFileSync } from "node:fs";
import { join } from "node:path";

import { repoRoot } from "./ports.ts";

// The pictures of the front page (docs/media/*.svg) cannot read the theme file; their colors are
// copied from it, and the tests of the pictures use these helpers to keep the copies the same.

const theme = readFileSync(join(repoRoot, "design/theme.kdl"), "utf8");

/** The text of a picture of `docs/media/`. */
export function readMedia(name: string): string {
  return readFileSync(join(repoRoot, "docs/media", name), "utf8");
}

/** The value of a token for one theme of `design/theme.kdl`, such as `#5b3fd9`. */
function tokenValue(token: string, themeName: string): string {
  const line = theme.split("\n").find((each) => each.trim().startsWith(`color "${token}" `));
  return new RegExp(` ${themeName}="(#[0-9a-f]{6})"`, "u").exec(line ?? "")?.[1] ?? "no such token";
}

/** The custom properties of a block of a picture, as `token: value` lines (`bg-canvas: #ffffff`). */
export function colorsOf(block: string): string[] {
  return (block.match(/--[a-z-]+: #[0-9a-f]{6};/gu) ?? []).map((line) => line.slice(2, -1));
}

/** What the tokens of a theme say about the same names, as `token: value` lines in the same order. */
export function tokensOf(block: string, themeName: string): string[] {
  return colorsOf(block).map((line) => {
    const token = line.slice(0, line.indexOf(":"));
    return `${token}: ${tokenValue(token, themeName)}`;
  });
}

/** The light and the dark block of colors of a picture. */
export function looksOf(picture: string): { light: string; dark: string } {
  const [light = "", afterLight = ""] = picture.split("@media (prefers-color-scheme: dark)");
  return { light, dark: afterLight.split("text {")[0] ?? "" };
}
