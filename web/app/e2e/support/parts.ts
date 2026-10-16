import { type Locator, type Page } from "@playwright/test";

/** An element of the app by the part it draws, as the theme file names it, and by its variant if it has one. */
export function partIn(scope: Locator | Page, name: string, variant?: string): Locator {
  const variantSelector = variant === undefined ? "" : `[data-variant="${variant}"]`;
  return scope.locator(`[data-part="${name}"]${variantSelector}`);
}
