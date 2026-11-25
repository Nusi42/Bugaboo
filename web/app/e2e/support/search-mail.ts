import type { Page } from "@playwright/test";

import { chooseTheme, themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { screenshotFolder, shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";

/**
 * The pictures of *E-mail me this search* (QRY-07): the open screen in the four themes, named for
 * the screen and the layout. Taken only when `BB_SCREENSHOT_DIR` is set.
 */
export async function mailPictures(page: Page, name: string): Promise<void> {
  if (screenshotFolder === undefined) {
    return;
  }
  const layout = layoutOf(page);
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shotWholeAs(page, `${name}-${layout}`, "M10/search-mail", theme.id);
    }),
  );
}
