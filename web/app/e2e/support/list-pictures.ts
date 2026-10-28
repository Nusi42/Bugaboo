import { t } from "@bugaboo/messages";
import { join } from "node:path";

import { type Locator, type Page } from "@playwright/test";

import { tickBox } from "./issues.ts";
import { setBox } from "./list-keys.ts";
import { chooseTheme, themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { folderOf, shootWindowAs, shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";

const MILESTONE = "M5/list";

/** Takes the open list in the four themes, named for the state it shows and the layout it is on. */
export async function listInFourThemes(page: Page, name: string): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shotWholeAs(page, `${name}-${layout}`, MILESTONE, theme.id);
    }),
  );
}

/**
 * The window in the four themes with the pointer on one row: the pointer is put back before each
 * picture, because a theme change or a grown window would take it away, and the hover line would be gone.
 */
export async function pointedRowInFourThemes(
  page: Page,
  row: Locator,
  name: string,
): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await row.hover();
      await page.waitForTimeout(300);
      await page.screenshot({
        path: join(folderOf(MILESTONE), `${name}-${layout}-${theme.id}.png`),
      });
    }),
  );
}

/** Waits until the page has drawn the theme it was given: two frames, not a guess of time. */
async function settled(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise<void>((done) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          done();
        });
      });
    });
  });
}

/** The same without growing the window to the list: what a person sees, for a list that reads more rows when it is taller. */
export async function listWindowInFourThemes(page: Page, name: string): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await settled(page);
      await shootWindowAs(page, `${name}-${layout}`, MILESTONE, theme.id);
    }),
  );
}

/** Ticks the boxes of these issues (by their IDs), as a person does. */
export async function tickIssues(page: Page, keys: readonly string[]): Promise<void> {
  await inOrder(keys.map((key) => () => setBox(tickBox(page, key), true)));
}

/** Ticks the box in the head of the list, which ticks the issues shown. */
export async function tickShown(page: Page): Promise<void> {
  await setBox(page.getByRole("checkbox", { name: t("issues.select-shown"), exact: true }), true);
}

// The folder the pictures go to: the one flag of every picture spec, re-exported so a spec that already imports this file takes no other import.
export { screenshotFolder } from "./screenshots.ts";
