import { basename, dirname, join } from "node:path";

import { expect, type Page } from "@playwright/test";

import { chooseTheme, themes } from "./checks.ts";
import { boxOnScreen, dragBy, handleOf } from "./dashboards.ts";
import { folderOf } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";

/**
 * Takes the picture of a box held in the middle of a drag by its title, in the four themes (`<path>-<theme>.png` below
 * the screenshot folder): each theme is chosen before the drag, the picture is taken while the box is held, then it is
 * let go.
 */
export async function shootHeldDrag(
  page: Page,
  title: string,
  steps: { readonly columns: number; readonly rows: number },
  path: string,
): Promise<void> {
  const [folder, name] = [dirname(path), basename(path)];
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await dragBy(page, handleOf(page, title), steps, false);
      await expect(boxOnScreen(page, title)).toHaveAttribute("data-state", "dragging");
      await page.screenshot({ path: join(folderOf(folder), `${name}-${theme.id}.png`) });
      await page.mouse.up();
    }),
  );
}
