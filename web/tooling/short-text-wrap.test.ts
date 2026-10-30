import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { repoRoot } from "./ports.ts";

/** The style sheet the theme file makes, as the browser gets it. */
const sheet = readFileSync(resolve(repoRoot, "web/packages/ui/src/theme.css"), "utf8");

/** The fewest selectors with a wrap the sheet has (the page, the short texts, the hints): fewer means it was not read. */
const FEWEST_WRAPPED_SELECTORS = 5;

/** The parts whose short texts are balanced (UX §7.3): the introduction, the preview, the answer titles, the hints. */
const balancedParts = [
  "card.intro",
  "access.preview-line",
  "access.subheading",
  "field.description",
  "settings.help",
] as const;

/** One rule of the sheet: where it stands (the @media and @supports around it), its selectors and its declarations. */
interface Rule {
  readonly context: string;
  readonly selectors: readonly string[];
  readonly body: string;
}

/** The value of each `text-wrap` that the rules of the sheet give a selector at the top level, in order. */
function topLevelWraps(css: string, selector: string): readonly string[] {
  const found: string[] = [];
  for (const rule of rulesOf(css)) {
    if (rule.context === "" && rule.selectors.includes(selector)) {
      for (const [, wrap = ""] of rule.body.matchAll(/text-wrap\s*:\s*([a-z-]+)/gu)) {
        found.push(wrap);
      }
    }
  }
  return found;
}

/**
 * Every rule of the sheet, at the top level and inside @media or @supports, with the blocks it stands in. A statement
 * that ends in `;` instead of a block (`@import`, `@charset`, `@layer a, b;`) is skipped, at the top level as inside a
 * block.
 */
function rulesOf(css: string): readonly Rule[] {
  const rules: Rule[] = [];
  const around: string[] = [];
  let start = 0;
  for (let at = 0; at < css.length; at += 1) {
    const char = css[at];
    if (char === "{") {
      const prelude = css.slice(start, at).trim();
      if (prelude.startsWith("@")) {
        around.push(prelude);
        start = at + 1;
      } else {
        const end = css.indexOf("}", at);
        rules.push({
          context: around.join(" "),
          selectors: prelude.split(",").map((selector) => selector.trim()),
          body: css.slice(at + 1, end),
        });
        at = end;
        start = end + 1;
      }
    } else if (char === "}") {
      around.pop();
      start = at + 1;
    } else if (char === ";" && css.slice(start, at).trim().startsWith("@")) {
      start = at + 1;
    }
  }
  return rules;
}

/** For each selector in each place, how many times the sheet sets its `text-wrap`. */
function wrapsBySelector(css: string): ReadonlyMap<string, number> {
  const counts = new Map<string, number>();
  for (const rule of rulesOf(css)) {
    const wraps = rule.body.match(/(?:^|;|\s)text-wrap\s*:/gu)?.length ?? 0;
    for (const selector of wraps > 0 ? rule.selectors : []) {
      const key = `${rule.context} ${selector}`.trim();
      counts.set(key, (counts.get(key) ?? 0) + wraps);
    }
  }
  return counts;
}

/** The selectors that the sheet gives more than one `text-wrap`, in the same place. */
function setTwice(css: string): readonly string[] {
  const twice: string[] = [];
  for (const [key, count] of wrapsBySelector(css)) {
    if (count > 1) {
      twice.push(key);
    }
  }
  return twice;
}

describe("ux_5 each text wrap is set once", () => {
  it("gives no selector more than one text-wrap, at the top level or inside @media", () => {
    // The sheet does set wraps (the page, the short texts, the hints), so the reading of it works.
    expect(wrapsBySelector(sheet).size).toBeGreaterThan(FEWEST_WRAPPED_SELECTORS);
    expect(setTwice(sheet)).toEqual([]);
  });

  it("finds a selector that gets two, so the check can fail", () => {
    const twice = '[data-part="x"] { text-wrap: balance; } [data-part="x"] { text-wrap: pretty; }';
    expect(setTwice(twice)).toEqual(['[data-part="x"]']);
  });

  it.each(balancedParts)("balances %s, once", (name) => {
    expect(topLevelWraps(sheet, `[data-part="${name}"]`)).toEqual(["balance"]);
  });

  it("leaves the words of a switch to the wrap of the page", () => {
    expect(wrapsBySelector(sheet).get('[data-part="switch"]')).toBeUndefined();
  });
});
