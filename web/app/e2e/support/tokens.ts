import { type Page } from "@playwright/test";

/** The value of a token on the page, as the browser works it out (`--space-4`). */
export function tokenValue(page: Page, name: string): Promise<string> {
  return page.evaluate(
    (token) => getComputedStyle(document.documentElement).getPropertyValue(token).trim(),
    name,
  );
}

/** A token that holds a length in pixels, as a number: the widths the specs compare with come from the theme file. */
export async function pixelsOf(page: Page, name: string): Promise<number> {
  return Number((await tokenValue(page, name)).replace("px", ""));
}
