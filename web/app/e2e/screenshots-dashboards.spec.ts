import { t } from "@bugaboo/messages";

import { paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { teamBoardAddress } from "./support/dashboard-issues.ts";
import { showAsTable } from "./support/dashboards.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The pictures of dashboards and totals (CFG-03, QRY-08): a dashboard, *Change*, *Add box* with its previews, a chart as its table, *Home* with its boxes, and the totals rows of a grouped list, in
// the four themes, on a computer and on a phone. Taken only when `BB_SCREENSHOT_DIR` is set; the builder looks at each.

test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");

test.use({ preparation: { run: prepareWeb } });

/** Where the pictures go, below the screenshot folder. */
const FOLDER = "M10/dashboards";

test("pictures of a dashboard, its chart as a table, Change and Add box", async ({
  page,
  server,
}) => {
  const address = await teamBoardAddress(server);
  await signIn(page, ada);
  await page.goto(address);
  await expect(page.getByRole("figure").first()).toBeVisible();
  await inFourThemes(page, "dashboard", FOLDER);
  await showAsTable(page.getByRole("figure").nth(1));
  await inFourThemes(page, "dashboard-table", FOLDER);
  await page.getByRole("button", { name: t("dashboards.change"), exact: true }).click();
  await inFourThemes(page, "dashboard-change", FOLDER);
  await page.getByRole("button", { name: t("dashboards.add-box"), exact: true }).click();
  await expect(page.getByRole("dialog", { name: t("dashboards.add-box") })).toBeVisible();
  await inFourThemesAsWindow(page, "add-box", FOLDER);
});

test("pictures of Home with its boxes and of the totals of a grouped list", async ({
  page,
  server,
}) => {
  await teamBoardAddress(server);
  await signIn(page, ada);
  await page.goto(paths.home);
  await expect(
    page.getByRole("button", { name: t("dashboards.customize"), exact: true }),
  ).toBeVisible();
  await inFourThemes(page, "home", FOLDER);
  await page.goto(
    `${projectIssuesAddress("WEB")}?group=priority&cols=status,id,title,estimated%20time`,
  );
  await expect(
    page.getByRole("row", {
      name: t("issues.totals-of", { group: t("priority.high") }),
      exact: false,
    }),
  ).toBeVisible();
  await inFourThemes(page, "list-totals", FOLDER);
});
