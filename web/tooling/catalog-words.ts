import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import type { Plugin } from "vite";

import { repoRoot } from "./ports.ts";

/** A message of the catalog in a page of the app: `{{app.name}}` in `index.html`. */
const placeholder = /\{\{([a-z][a-z0-9.-]*)\}\}/gu;

/** The catalog that every word of the product comes from (UX-07). */
const catalogPath = resolve(repoRoot, "messages/en.json");

/** The page with every `{{key}}` replaced by the message of that key. */
export function putWordsIn(html: string, catalog: unknown): string {
  return html.replaceAll(placeholder, (_, key: string) => {
    const text: unknown =
      typeof catalog === "object" && catalog !== null ? Reflect.get(catalog, key) : undefined;
    if (typeof text !== "string") {
      throw new TypeError(`index.html names the message ${key}, which messages/en.json lacks.`);
    }
    return text;
  });
}

/**
 * Puts the words of the message catalog into `index.html` while it is built (M0-F04), so the
 * static page, which cannot call `t()`, still repeats nothing: the name in the title of the tab
 * comes from `messages/en.json`, like every other word.
 */
export function catalogWords(): Plugin {
  return {
    name: "bugaboo-catalog-words",
    transformIndexHtml(html: string): string {
      return putWordsIn(html, JSON.parse(readFileSync(catalogPath, "utf8")));
    },
  };
}
