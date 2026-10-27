import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./accounts.ts";
import { chooseTheme, themes } from "./checks.ts";
import { expect } from "./fixtures.ts";
import { signIn } from "./flows.ts";
import { layoutOf, onThisLayout } from "./layout.ts";
import { shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";

const MILESTONE = "M5/search";

/** Takes the open screen in the four themes, named for the screen and the window it is shown in. */
export async function inFourThemes(page: Page, name: string): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shotWholeAs(page, `${name}-${layout}`, MILESTONE, theme.id);
    }),
  );
}

export async function openSearch(page: Page, text: string): Promise<void> {
  await signIn(page, ada);
  await page.goto(`/search?s=${encodeURIComponent(text)}`);
  await expect(page.getByRole("heading", { level: 1, name: t("search.page.title") })).toBeVisible();
}

/** On a phone the bar is one button that opens a sheet. */
export async function openBar(page: Page): Promise<void> {
  if (layoutOf(page) === "phone") {
    await page.getByRole("button", { name: /^Filters/u }).click();
  }
}

/** The key `/` went to the bar: to its first control on a computer, into its sheet on a phone. */
export async function expectBarKeyWorked(page: Page): Promise<void> {
  const first = async (): Promise<void> => {
    await expect(page.getByRole("button", { name: t("search.bar.add-filter") })).toBeFocused();
  };
  await onThisLayout(page, {
    computer: first,
    tablet: first,
    phone: async () => {
      await expect(page.getByRole("dialog", { name: t("search.bar.filters") })).toBeVisible();
    },
  });
}
