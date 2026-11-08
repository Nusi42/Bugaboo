import { expect, type Locator } from "@playwright/test";

import { topOf } from "./geometry.ts";

/** How many readings in a row must agree for an element to count as standing still. */
const STILL_READINGS = 5;

/** Waits until the top of an element has stood still for a few readings: a window slides in on a phone. */
export async function waitUntilStill(element: Locator): Promise<void> {
  const places: number[] = [];
  await expect
    .poll(async () => {
      // A window that is not on the screen yet starts the count again.
      const place = await topOf(element).catch(() => Number.NaN);
      if (Number.isNaN(place)) {
        places.length = 0;
        return false;
      }
      places.push(place);
      return new Set(places.slice(-STILL_READINGS)).size === 1 && places.length >= STILL_READINGS;
    })
    .toBe(true);
}
