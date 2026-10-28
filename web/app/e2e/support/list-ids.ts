import { expect, type Locator, type Page } from "@playwright/test";

import { variants } from "../../../packages/ui/src/tokens.ts";
import { ada } from "./accounts.ts";
import { signIn } from "./flows.ts";
import { grid } from "./list-look.ts";
import { partIn } from "./parts.ts";

// The ID column of the issue list (UX §4.2): as wide as the longest ID it draws.

/** How many of the IDs drawn in the list are cut: their cell holds less than they are wide. */
export function idsCut(page: Page): Promise<number> {
  return page.evaluate(() => {
    let count = 0;
    for (const cell of document.querySelectorAll(
      "[data-part='list-row.cell'][data-variant='key']",
    )) {
      if (cell.scrollWidth > cell.clientWidth) {
        count += 1;
      }
    }
    return count;
  });
}

/** How wide the cells of the ID column are, in pixels. */
export function keyWidth(list: Locator): Promise<number> {
  return partIn(list, "list-row.cell", "key")
    .first()
    .evaluate((cell) => cell.getBoundingClientRect().width);
}

/** Signs in as the administrator, opens the list of every issue and waits for its first ID. */
export async function openEveryList(page: Page): Promise<void> {
  await signIn(page, ada);
  await page.goto("/search?group=-");
  await expect(partIn(grid(page), "list-row.cell", "key").first()).toBeVisible();
}

/** Sets an attribute of the page, as the settings of the appearance do. */
function setPageAttribute([name, value]: readonly [string, string]): void {
  document.documentElement.setAttribute(name, value);
}

/** Turns on *Larger text* for the open page (the text scale 1.25 of the theme file). */
export function useLargerText(page: Page): Promise<void> {
  const { attribute, value } = variants["larger-text"];
  return page.evaluate(setPageAttribute, [attribute, value] as const);
}
