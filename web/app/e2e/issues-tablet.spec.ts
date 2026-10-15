import { t } from "@bugaboo/messages";

import { issuePanelAddress } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb, webIssues } from "./support/issues.ts";

// An issue on a tablet (UX §3.2, M3-F57): held upright it opens over the list as a full-height
// sheet; turned sideways the window has room for the side panel.

test.use({ preparation: { run: prepareWeb } });

test("ux_03 held upright, a tablet opens the issue as a full-height sheet over the list, and sideways it has the panel (M3-F57)", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.setViewportSize(viewports.tablet);
  await page.goto(issuePanelAddress("WEB", "WEB-2"));
  const sheet = page.getByRole("dialog", { name: t("issues.sheet"), exact: true });
  await expect(sheet).toBeVisible();
  await expect(
    sheet.getByRole("heading", { name: new RegExp(webIssues.passkey, "u") }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
  // Esc closes the sheet and takes the issue out of the address; the list is there again.
  await page.keyboard.press("Escape");
  await expect(sheet).toHaveCount(0);
  await expect(page).toHaveURL(/\/issues$/u);
  // Turned sideways the same window has room for the panel next to the list.
  await page.setViewportSize(viewports.tabletSideways);
  await page.goto(issuePanelAddress("WEB", "WEB-2"));
  await expect(page.getByRole("complementary", { name: "WEB-2", exact: true })).toBeVisible();
  await expect(sheet).toHaveCount(0);
});
