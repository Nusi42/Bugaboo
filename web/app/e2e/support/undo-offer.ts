import { expect, type Page } from "@playwright/test";

import { partIn } from "./parts.ts";
import { boxOf } from "./windows.ts";

/** The offer to undo does not cover the Details: the two boxes lie apart. */
export async function expectOfferApartFromDetails(page: Page): Promise<void> {
  const offer = await boxOf(partIn(page, "undo-host.offer"));
  const details = await boxOf(partIn(page, "name-values"));
  const apart =
    offer.x + offer.width <= details.x ||
    details.x + details.width <= offer.x ||
    offer.y + offer.height <= details.y ||
    details.y + details.height <= offer.y;
  expect(apart).toBe(true);
}

/**
 * The offer to undo ends before the column of the Details starts, wherever the page is scrolled to:
 * a long message wraps inside that room and never reaches over the Details.
 */
export async function expectOfferBesideTheDetails(page: Page): Promise<void> {
  const offer = await boxOf(partIn(page, "undo-host.offer"));
  const details = await boxOf(partIn(page, "name-values"));
  expect(offer.x + offer.width).toBeLessThanOrEqual(details.x);
}
