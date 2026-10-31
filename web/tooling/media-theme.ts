import { readFileSync } from "node:fs";
import { join } from "node:path";

import { repoRoot } from "./ports.ts";

import { pageAttributes } from "../packages/ui/src/tokens.ts";

// The pictures of the front page (docs/media/*.svg) cannot read the theme; their colors are copied
// from it, and the tests of the pictures use these helpers to keep the copies the same. The colors
// are read from the style sheet that `cargo xtask gen theme` writes from the theme files, so KDL
// has one reader (ADR-0026).

const sheet = readFileSync(join(repoRoot, "web/packages/ui/src/theme.css"), "utf8");

/** The text of a picture of `docs/media/`. */
export function readMedia(name: string): string {
  return readFileSync(join(repoRoot, "docs/media", name), "utf8");
}

/** The variables of one theme as the sheet sets them on a page that shows it, by name. */
function themeVariables(themeName: string): ReadonlyMap<string, string> {
  const start = sheet.indexOf(`:root[${pageAttributes.look}="${themeName}"]`);
  const block = start < 0 ? "" : sheet.slice(start, sheet.indexOf("}", start));
  const variables = new Map<string, string>();
  for (const found of block.matchAll(/--([a-z0-9-]+): ([^;]+);/gu)) {
    variables.set(found[1] ?? "", found[2] ?? "");
  }
  return variables;
}

/** The value of a token for one theme, such as `#5b3fd9`, with a token that names another followed. */
function tokenValue(token: string, themeName: string): string {
  const variables = themeVariables(themeName);
  // The color a variable leads to, at most `links` links deep: a chain of links is never longer
  // than the list, because the generator refuses a loop.
  const colorOf = (name: string, links: number): string => {
    const written = variables.get(name) ?? "";
    const link = /^var[(]--([a-z0-9-]+)[)]$/u.exec(written)?.[1];
    if (link !== undefined && links > 0) {
      return colorOf(link, links - 1);
    }
    return /^#[0-9a-f]{6}$/u.test(written) ? written : "no such token";
  };
  return colorOf(token, variables.size);
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
