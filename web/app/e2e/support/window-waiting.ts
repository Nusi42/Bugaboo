import { expect, type Page } from "@playwright/test";

import { part } from "../../../packages/ui/src/tokens.ts";
import { placesWhile, type Place } from "./frames.ts";
import { layoutOf } from "./layout.ts";
import { partIn, partSelector } from "./parts.ts";

/** Holds back the answers to the requests that match, as a slow network would, until the returned function is called. */
async function holdAnswers(page: Page, requests: string): Promise<() => void> {
  const held = Promise.withResolvers<void>();
  await page.route(requests, async (route) => {
    await held.promise;
    await route.continue();
  });
  return held.resolve;
}

/** Holds back the answers about the layout of the new-issue form. */
export function holdNewIssueLayout(page: Page): Promise<() => void> {
  return holdAnswers(page, "**/api/v1/layouts/form?*kind=new-issue*");
}

/**
 * Holds back the code of the new-issue window, which the browser fetches when the window is first opened (or a while
 * after the page is up). Set it before the page loads, so the early fetch is held too.
 */
export function holdNewIssueCode(page: Page): Promise<() => void> {
  return holdAnswers(page, "**/assets/NewIssueDialog-*.js");
}

/** Waits until the open window can be seen: for its first 0.3 s a window that is loading is there but clear. */
export async function expectWindowSeen(page: Page): Promise<void> {
  await expect(partIn(page, part.overlayModal)).toHaveCSS("opacity", "1");
}

/** Whether the open window is clear right now: read once, not waited for. */
export function windowIsClear(page: Page): Promise<boolean> {
  return partIn(page, part.overlayModal).evaluate((box) => getComputedStyle(box).opacity === "0");
}

/** Where the open window stands in every frame in which it is shown while `act` runs (see `placesWhile`). */
export function windowPlacesWhile(page: Page, act: () => Promise<void>): Promise<readonly Place[]> {
  const variant = layoutOf(page) === "phone" ? "full" : "window";
  return placesWhile(page, partSelector(part.overlayModal, variant), act);
}
