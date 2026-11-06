import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inFourChosenThemesAsWindow } from "./support/four-themes.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { chipOf, scene } from "./support/time-scene.ts";

// The sidebar in a window 600 px high with the timer and the work day running: closed, and with the
// group at the bottom open, which holds the Timesheet (M6/time). A spec for a computer.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

const MILESTONE = "M6/time";
const WINDOW_HEIGHT = 600;

test("the sidebar of a window 600 px high, closed and with its group open", async ({
  page,
  server,
}) => {
  // The timer runs and the work day too: the highest the sidebar gets.
  const { issue } = await scene(server);
  await page.setViewportSize({ width: 1280, height: WINDOW_HEIGHT });
  await signIn(page, ada);
  const open = async (): Promise<void> => {
    await page.goto("/");
    await expect(chipOf(page, issue.key)).toBeVisible();
    await expect(page.getByText(/^Working since/u)).toBeVisible();
  };
  await inFourChosenThemesAsWindow(page, "sidebar-600-closed", MILESTONE, paths.appearance, open);
  await inFourChosenThemesAsWindow(
    page,
    "sidebar-600-open",
    MILESTONE,
    paths.appearance,
    async () => {
      await open();
      await page.getByRole("button", { name: t("shell.group-fold"), exact: true }).click();
      await expect(
        page.getByRole("dialog", { name: t("shell.group-fold") }).getByRole("link", {
          name: t("nav.timesheet"),
          exact: true,
        }),
      ).toBeVisible();
    },
  );
});
