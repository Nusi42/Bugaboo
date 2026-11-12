import { expect, type Page } from "@playwright/test";

import { type Theme } from "./checks.ts";
import { everyScreen, screenValues, type ScreenValues } from "./every-screen.ts";
import { layoutOf } from "./layout.ts";
import { shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";
import { prepareAgent } from "./swipe-lists.ts";
import { changedPictures, TOUCH_PICTURES } from "./touch-changed-pictures.ts";

/** The name of a screen in the file names: its route without slashes and changing parts (`settings-people-person`). */
function slugOf(path: string): string {
  const words = path.replaceAll("$", "").split("/").filter(Boolean);
  return words.length === 0 ? "home" : words.join("-");
}

/** What the pictures open on the sample: the values of the addresses, and an agent for *My agents*. */
export async function prepareTouchPictures(server: Instance): Promise<ScreenValues> {
  const values = await screenValues(server);
  await prepareAgent(server);
  return values;
}

/**
 * Takes every screen of the route table in the theme the person chose, named `screen-<route>-<layout>-<theme>`, then on
 * a phone what changed (`changedPictures`).
 */
export async function takeTouchPictures(
  page: Page,
  values: ScreenValues,
  theme: Theme,
): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    everyScreen(values).map((screen) => async () => {
      await page.goto(screen.address);
      await expect(page.getByRole("main")).toBeVisible();
      await page.waitForLoadState("networkidle");
      await shotWholeAs(page, `screen-${slugOf(screen.path)}-${layout}`, TOUCH_PICTURES, theme.id);
    }),
  );
  // Only a phone swipes, and only a phone keeps the buttons of a form out of its footer.
  if (layout === "phone") {
    await changedPictures(page, theme);
  }
}
