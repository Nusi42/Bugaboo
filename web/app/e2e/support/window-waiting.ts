import { expect, type Page } from "@playwright/test";

import { part } from "../../../packages/ui/src/tokens.ts";
import { placesWhile, type Place } from "./frames.ts";
import { layoutOf } from "./layout.ts";
import { partIn, partSelector } from "./parts.ts";

/**
 * Holds back the answers about the layout of the new-issue form, as a slow server would, until the returned function is
 * called.
 */
export async function holdNewIssueLayout(page: Page): Promise<() => void> {
  const held = Promise.withResolvers<void>();
  await page.route("**/api/v1/layouts/form?*kind=new-issue*", async (route) => {
    await held.promise;
    await route.continue();
  });
  return held.resolve;
}

/** Waits until the open window can be seen: for its first 0.3 s a window that is loading is there but clear. */
export async function expectWindowSeen(page: Page): Promise<void> {
  await expect(partIn(page, part.overlayModal)).toHaveCSS("opacity", "1");
}

/** Where the open window stands in every frame in which it is shown while `act` runs (see `placesWhile`). */
export function windowPlacesWhile(page: Page, act: () => Promise<void>): Promise<readonly Place[]> {
  const variant = layoutOf(page) === "phone" ? "full" : "window";
  return placesWhile(page, partSelector(part.overlayModal, variant), act);
}
