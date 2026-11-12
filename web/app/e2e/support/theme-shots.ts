import { type Page } from "@playwright/test";

import { putOnTheme, themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";

export { inFourThemes, inFourThemesAsWindow } from "./four-themes.ts";

/**
 * Takes the open screen in the four themes with the page put on each theme directly: on a page with
 * theme pickers, whose samples are radios named like the themes, taking a picture chooses nothing.
 */
export async function inFourThemesPutOn(page: Page, name: string, folder: string): Promise<void> {
  await inOrder(
    themes.map((theme) => async () => {
      await putOnTheme(page, theme);
      await shotWholeAs(page, `${name}-${layoutOf(page)}`, folder, theme.id);
    }),
  );
}
