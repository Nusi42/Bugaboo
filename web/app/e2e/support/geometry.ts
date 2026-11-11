import { type Locator } from "@playwright/test";

/** How far the top of what the locator shows is from the top of the page, in whole pixels. */
export async function topOf(shown: Locator): Promise<number> {
  const box = await shown.boundingBox();
  if (box === null) {
    throw new TypeError("The element is not on the screen.");
  }
  return Math.round(box.y);
}

/** The tops of every element the locator shows, in whole pixels. */
export async function topsOf(shown: Locator): Promise<readonly number[]> {
  return Promise.all((await shown.all()).map((each) => topOf(each)));
}

/**
 * How far the top of what the locator shows is below the foot of the first part named `above` (a
 * bar): not less than nothing when it stands whole under it.
 */
export function gapUnder(shown: Locator, above: string): Promise<number> {
  return shown.evaluate((node, name) => {
    const foot = document.querySelector(`[data-part="${name}"]`)?.getBoundingClientRect().bottom;
    return node.getBoundingClientRect().top - (foot ?? Number.POSITIVE_INFINITY);
  }, above);
}
