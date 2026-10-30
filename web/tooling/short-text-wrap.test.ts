import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { repoRoot } from "./ports.ts";

/** The style sheet the theme file makes, as the browser gets it. */
const sheet = readFileSync(resolve(repoRoot, "web/packages/ui/src/theme.css"), "utf8");

/** The parts whose short texts are balanced (UX §7.3: no word alone on a last line). */
const balanced = [
  "card.intro",
  "access.preview-line",
  "access.subheading",
  "field.description",
] as const;

/** Every `text-wrap` the sheet gives a part in a rule that names the part itself (not a part inside it). */
function wrapsOf(name: string): readonly string[] {
  const own = `[data-part="${name}"]`;
  const found: string[] = [];
  for (const [, selectors = "", body = ""] of sheet.matchAll(/([^{}]+)\{([^{}]*)\}/gu)) {
    const names = selectors.split(",").map((selector) => selector.trim());
    if (names.includes(own)) {
      for (const [, wrap = ""] of body.matchAll(/text-wrap:\s*([a-z-]+)/gu)) {
        found.push(wrap);
      }
    }
  }
  return found;
}

describe("ux_5 short texts are balanced, set once", () => {
  it.each(balanced)("%s gets one text-wrap, balance", (name) => {
    expect(wrapsOf(name)).toEqual(["balance"]);
  });
});
