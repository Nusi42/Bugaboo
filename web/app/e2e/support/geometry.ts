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
