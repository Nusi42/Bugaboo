import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { layoutOf } from "./support/layout.ts";
import { openSearch } from "./support/search-bar.ts";
import { sampleAdministrator } from "./support/sample.ts";

// M5 exit, package O: the button that takes a filter away stays inside the border of its chip, also where the chips
// have little room: the words before it give way.

test.use({ sample: true });

/** How many chips have their last button outside their border. */
function chipsWithButtonOutside(area: Locator): Promise<number> {
  return area.evaluate((node) => {
    let outside = 0;
    for (const chip of node.querySelectorAll("[data-part='search-filter']")) {
      const last = chip.lastElementChild;
      if (
        last !== null &&
        last.getBoundingClientRect().right > chip.getBoundingClientRect().right + 0.5
      ) {
        outside += 1;
      }
    }
    return outside;
  });
}

test("ux_4 the button that takes a filter away is inside the border of its chip", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "a phone shows the filters in a sheet");
  await page.setViewportSize({ width: 1000, height: 800 });
  await openSearch(
    page,
    "status group is not Done and priority is high and assigned to is me",
    sampleAdministrator(),
  );
  const bar = page.locator("main");
  await expect(page.locator("[data-part='search-filter']").first()).toBeVisible();
  expect(await chipsWithButtonOutside(bar), "chips with the button outside").toBe(0);
});
