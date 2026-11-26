import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { teamBoardAddress } from "./support/dashboard-issues.ts";
import { shootHeldDrag } from "./support/dashboard-pictures.ts";
import { readyMadeTitle } from "./support/dashboards.ts";
import { test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The picture of a box of a dashboard dragged on a computer (CFG-03, UX §6.4): the boxes stay, the shadow shows where
// it lands, in the four themes. Taken only when `BB_SCREENSHOT_DIR` is set; the builder looks at each.

test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");

test.use({ preparation: { run: prepareWeb } });

/** Where the pictures go, below the screenshot folder. */
const FOLDER = "M10/dashboards";

test("pictures of a box dragged with its shadow", async ({ page, server }) => {
  const address = await teamBoardAddress(server);
  await signIn(page, ada);
  await page.goto(address);
  await page.getByRole("button", { name: t("dashboards.change"), exact: true }).click();
  const title = await readyMadeTitle(apiOf(page, server), "time-on-this-search");
  await shootHeldDrag(page, title, { columns: -6, rows: -2 }, `${FOLDER}/dashboard-drag-computer`);
});
