import { expect, type Page } from "@playwright/test";

import { doNothing } from "../../src/shared/do-nothing.ts";

/** How long every read of the app waits here: longer than the wait of the grey shapes, so a hold draws them. */
const SLOW_READ_MS = 2500;

/** Makes every read of the app answer late, from now on. */
export async function readSlowly(page: Page): Promise<void> {
  await page.route("**/api/v1/**", async (route) => {
    await new Promise((done) => {
      setTimeout(done, SLOW_READ_MS);
    });
    // The page may be gone by then: what it asked has nobody left to answer.
    await route.continue().catch(doNothing);
  });
}

/**
 * Opens `address` with slow reads and, once a hold draws its grey shapes (or none waits), says each set of shapes that
 * does not lie within the room its hold keeps, by the part the hold stands in.
 */
export async function shapesOutsideTheirRoom(page: Page, address: string): Promise<string[]> {
  await page.goto(address, { waitUntil: "commit" });
  // Until a hold draws its shapes, or nothing waits any more (a screen whose reads were in already draws none).
  await expect
    .poll(
      () =>
        page.evaluate(
          () =>
            document.querySelector("[data-part='reading-hold.shapes']") !== null ||
            (document.querySelector("main") !== null &&
              document.querySelector("[data-part='reading-hold'][data-state='held']") === null),
        ),
      { timeout: 8 * SLOW_READ_MS },
    )
    .toBe(true);
  return page.evaluate(() => {
    const wrong: string[] = [];
    for (const shapes of document.querySelectorAll("[data-part='reading-hold.shapes']")) {
      let hold = shapes.nextElementSibling;
      while (hold instanceof HTMLElement && hold.dataset["part"] !== "reading-hold") {
        hold = hold.nextElementSibling;
      }
      const range = document.createRange();
      range.selectNodeContents(hold ?? shapes);
      const room = range.getBoundingClientRect();
      const drawn = shapes.getBoundingClientRect();
      const inside =
        drawn.left >= room.left - 1 &&
        drawn.right <= room.right + 1 &&
        drawn.top >= room.top - 1 &&
        drawn.bottom <= room.bottom + 1;
      if (!inside) {
        wrong.push(
          `${hold?.parentElement?.dataset["part"] ?? "?"}: shapes ${JSON.stringify(drawn)} room ${JSON.stringify(room)}`,
        );
      }
    }
    return wrong;
  });
}
