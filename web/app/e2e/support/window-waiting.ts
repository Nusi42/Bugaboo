import { expect, type Page } from "@playwright/test";

import { part } from "../../../packages/ui/src/tokens.ts";
import { placesWhile, type Place } from "./frames.ts";
import { layoutOf } from "./layout.ts";
import { partIn, partSelector } from "./parts.ts";

/** A hold of the answers to some requests: how often they were asked for, and the way to let them through. */
interface Hold {
  readonly asked: () => number;
  readonly release: () => void;
}

/** Holds back the answers to the requests that match, as a slow network would, until it is released. */
async function holdAnswers(page: Page, requests: string): Promise<Hold> {
  const held = Promise.withResolvers<void>();
  let asked = 0;
  await page.route(requests, async (route) => {
    asked += 1;
    await held.promise;
    await route.continue();
  });
  return { asked: () => asked, release: held.resolve };
}

/** Holds back the answers about the layout of the new-issue form. */
export async function holdNewIssueLayout(page: Page): Promise<() => void> {
  return (await holdAnswers(page, "**/api/v1/layouts/form?*kind=new-issue*")).release;
}

/** Where the browser fetches the code of the new-issue window. */
const NEW_ISSUE_CODE = "**/assets/NewIssueDialog-*.js";

/**
 * Holds back the code of the new-issue window, which the browser fetches when the window is first opened (or a while
 * after the page is up). Set it before the page loads, so the early fetch is held too. The release waits until the code
 * was asked for and held: a pattern that matched nothing (a renamed file) would hold nothing back and prove nothing.
 */
export async function holdNewIssueCode(page: Page): Promise<() => Promise<void>> {
  const hold = await holdAnswers(page, NEW_ISSUE_CODE);
  return async () => {
    await expect
      .poll(hold.asked, { message: `${NEW_ISSUE_CODE} is asked for and held` })
      .toBeGreaterThan(0);
    hold.release();
  };
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
