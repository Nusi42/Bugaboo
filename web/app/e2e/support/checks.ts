import { AxeBuilder } from "@axe-core/playwright";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { inOrder } from "./sequence.ts";

/** The four built-in themes, by the id on the page and the name people read. */
export const themes = [
  { id: "daylight", label: t("theme.daylight") },
  { id: "paper", label: t("theme.paper") },
  { id: "dusk", label: t("theme.dusk") },
  { id: "midnight", label: t("theme.midnight") },
] as const;

export type Theme = (typeof themes)[number];

/** Picks a theme with the choice at the bottom of the page, as a person would. */
export async function chooseTheme(page: Page, theme: Theme): Promise<void> {
  await page.getByRole("radio", { name: theme.label, exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute("data-look", theme.id);
}

/**
 * Puts a theme on the page without the choice at the bottom, which a question that covers the
 * page makes unreachable. The colors follow `data-look` at once.
 */
async function putOnTheme(page: Page, theme: Theme): Promise<void> {
  await page.evaluate((look) => {
    document.documentElement.dataset["look"] = look;
  }, theme.id);
}

/** The screen has no accessibility violation that the checks of axe can find (WCAG 2.2 AA). */
export async function expectNoViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  expect(results.violations).toEqual([]);
}

/** The page is not wider than the window, so nobody has to scroll sideways. */
export async function expectNoSidewaysScroll(page: Page): Promise<void> {
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
}

/** The open screen is accessible and does not scroll sideways, in every theme, picked as a person does. */
export function expectGoodInEveryTheme(page: Page): Promise<void> {
  return inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await expectNoViolations(page);
      await expectNoSidewaysScroll(page);
    }),
  );
}

/** The same without the choice of theme: for screens with a question open over the page. */
export function expectGoodInEveryLook(page: Page): Promise<void> {
  return inOrder(
    themes.map((theme) => async () => {
      await putOnTheme(page, theme);
      await expectNoViolations(page);
      await expectNoSidewaysScroll(page);
    }),
  );
}
