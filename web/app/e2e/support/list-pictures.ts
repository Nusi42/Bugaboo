import { type Page } from "@playwright/test";

import { chooseTheme, themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { shotWholeAs } from "./screenshots.ts";
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
