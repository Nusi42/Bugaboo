import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { inFourThemesOf } from "./support/choice-pictures.ts";
import { costCenterFieldAddress, prepareCostCenters } from "./support/cost-centers.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The numbered cost centers in the four themes, for the review of M6: the field page with numbers,
// a number used twice, the box of a number with focus and with an error,. They are only taken when `BB_SCREENSHOT_DIR` names the folder
// they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareCostCenters } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M6/choice-numbers";

/** The choice *Overhead*, the last of the list: the one without a number. */
function overheadOf(page: Page) {
  return page
    .getByRole("list", { name: t("fields.choices"), exact: true })
    .getByRole("listitem")
    .last();
}

test("the field Cost center with numbers", async ({ page, server }) => {
  const address = await costCenterFieldAddress(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "fields-page", MILESTONE, paths.appearance, async () => {
    await page.goto(address);
    await expect(page.getByRole("heading", { level: 1, name: "Cost center" })).toBeVisible();
    await expect(page.locator("[data-part='choice-number']").first()).toBeVisible();
  });
});

test("a number used twice is told and not saved, and the box of a number with focus", async ({
  page,
  server,
}) => {
  const address = await costCenterFieldAddress(server);
  await signIn(page, ada);
  const box = () => overheadOf(page).getByLabel(t("fields.choice-number"), { exact: true });
  const save = () => overheadOf(page).getByRole("button", { name: t("common.save"), exact: true });
  await inFourThemesOf(
    page,
    () => overheadOf(page),
    { name: "number-focus", milestone: MILESTONE },
    async () => {
      await page.goto(address);
      await box().focus();
      await page.keyboard.press("End");
    },
  );
  await inFourThemesOf(
    page,
    () => overheadOf(page),
    { name: "number-used-twice", milestone: MILESTONE },
    async () => {
      await page.goto(address);
      await box().fill("4100");
      await save().click();
      await expect(overheadOf(page)).toContainText(t("error.choice.number-taken"));
    },
  );
  await inFourThemesOf(
    page,
    () => overheadOf(page),
    { name: "number-not-digits", milestone: MILESTONE },
    async () => {
      await page.goto(address);
      await box().fill("K-100");
      await save().click();
      await expect(overheadOf(page)).toContainText(t("fields.choice-number-invalid", { max: 9 }));
    },
  );
});
