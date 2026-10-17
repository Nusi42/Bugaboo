import { type Page } from "@playwright/test";

import { chooseTheme, themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";

/** Where the pictures of notifications go, inside the folder the run was given. */
const MILESTONE = "M4/notifications";

/** Takes the open screen in every theme, with the size of the window in the name of the picture. */
export async function inEveryTheme(page: Page, name: string): Promise<void> {
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shotWholeAs(page, `${name}-${layoutOf(page)}`, MILESTONE, theme.id);
    }),
  );
}
