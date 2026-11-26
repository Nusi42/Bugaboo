import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { cardOf, mapList, mapped, openMap, prepareRelationMap } from "./support/relation-map.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The relation map in the four themes, on a computer and on a phone (ISS-16, UX §4.15): the picture
// with the chain of a chosen issue, the list form, and *Show as › Map* of a search. Only taken when
// `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-relation-map.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareRelationMap } });

const MILESTONE = "M10/relation-map";

/** Shows the list: a computer starts with the picture. */
async function showList(page: Page): Promise<void> {
  if (layoutOf(page) !== "phone") {
    await page.getByRole("radio", { name: t("relation-map.list"), exact: true }).check();
  }
  await expect(mapList(page)).toBeVisible();
}

/** Shows the picture: a phone starts with the list. */
async function showPicture(page: Page): Promise<void> {
  if (layoutOf(page) === "phone") {
    await page
      .getByRole("button", { name: t("relation-map.show-as-picture"), exact: true })
      .click();
  }
  await expect(cardOf(page, mapped.start)).toBeVisible();
}

test("the picture of the relation map with a chain chosen", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await showPicture(page);
  await cardOf(page, mapped.timesheet).click();
  // The click brought the card into view; the picture is taken from its start, as it opens.
  await page.locator('[data-part="map-scroll"]').evaluate((box) => {
    box.scrollTo(0, 0);
  });
  await inFourThemes(page, "picture-chain", MILESTONE);
});

test("the picture of the relation map", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await showPicture(page);
  await inFourThemes(page, "picture", MILESTONE);
});

test("the list form of the relation map", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await showList(page);
  await inFourThemes(page, "list", MILESTONE);
});

test("a search shown as a map", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/search?s=project%20is%20WEB&show=map");
  await showPicture(page);
  await inFourThemes(page, "search-map", MILESTONE);
});
