import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { tickBox } from "./issues.ts";
import { setBox } from "./list-keys.ts";
import { chooseTheme, themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { screenshotFolder, shootWindowAs, shotWholeAs } from "./screenshots.ts";

/** Whether the pictures are wanted: `BB_SCREENSHOT_DIR` is set. */
export const picturesWanted = screenshotFolder !== undefined;
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
