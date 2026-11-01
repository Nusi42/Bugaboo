import { type Page } from "@playwright/test";

import { chooseTheme, themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";
import { showTheme } from "./showcase.ts";

/** Where the pictures go, below the folder the run was given, unless a test names its own. */
const MILESTONE = "M4/workflows";

/**
 * Takes the open screen in the four themes, named for the screen and for the window it is shown in,
 * into the folder `milestone` below the one the run was given.
 */
export async function inFourThemes(page: Page, name: string, milestone = MILESTONE): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shotWholeAs(page, `${name}-${layout}`, milestone, theme.id);
    }),
  );
}

/**
 * The same for a screen that reads the person's appearance again when it opens (the issue page):
 * each theme is chosen in *My settings › Appearance* as a person does, then `open` opens the screen.
 */
export async function inFourChosenThemes(
  page: Page,
  name: string,
  milestone: string,
  appearance: string,
  open: () => Promise<void>,
): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    themes.map((theme) => async () => {
      await showTheme(page, theme, appearance);
      await open();
      await shotWholeAs(page, `${name}-${layout}`, milestone, theme.id);
    }),
  );
}
