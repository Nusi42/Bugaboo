import { type Locator, type Page } from "@playwright/test";

/**
 * The values of `aria-current`, from their one home in the app, for the tests that read the marks of parts. Taken from
 * the file itself, not the package's index: the load check runs these helpers in plain Node, which cannot read the
 * components the index brings along.
 */
export { ariaCurrent } from "@bugaboo/ui/aria-current";

/** An element of the app by the part it draws, as the theme file names it, and by its variant if it has one. */
export function partIn(scope: Locator | Page, name: string, variant?: string): Locator {
  const variantSelector = variant === undefined ? "" : `[data-variant="${variant}"]`;
  return scope.locator(`[data-part="${name}"]${variantSelector}`);
}

/** The row of a field in the Details of the issue page, found by the name of the field. */
export function detailRow(page: Page, name: string): Locator {
  return partIn(page, "name-value").filter({ has: page.getByText(name, { exact: true }) });
}
