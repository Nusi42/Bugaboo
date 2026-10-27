import { type Page } from "@playwright/test";

import { chooseTheme, themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";

/** Takes the open screen in the four themes, named for the screen and for the window it is shown in. */
export async function inFourThemes(page: Page, name: string): Promise<void> {
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shotWholeAs(page, `${name}-${layoutOf(page)}`, "M5/quick-search", theme.id);
    }),
  );
}
