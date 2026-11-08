import { expect, type Locator } from "@playwright/test";

import { topOf } from "./geometry.ts";

/** How many readings in a row must agree for an element to count as standing still. */
const STILL_READINGS = 5;

/** Waits until the top of an element has stood still for a few readings: a window slides in on a phone. */
export async function waitUntilStill(element: Locator): Promise<void> {
  const places: number[] = [];
  await expect
    .poll(async () => {
      places.push(await topOf(element));
      return new Set(places.slice(-STILL_READINGS)).size === 1 && places.length >= STILL_READINGS;
    })
    .toBe(true);
}
