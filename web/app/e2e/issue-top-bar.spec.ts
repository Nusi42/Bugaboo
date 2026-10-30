import { expect } from "@playwright/test";

import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { issueAddress } from "../src/shared/paths.ts";

// M5 exit, package O: the top bar of an issue page on a computer says where you are, like every other page: the
// project, then the issue.

test.use({ sample: true });

test("ux_8 the top bar of the issue page has the project and the issue ID", async ({ page }) => {
  test.skip(layoutOf(page) !== "computer", "a phone has the bars of the focus view");
  await signIn(page, sampleAdministrator());
  await page.goto(issueAddress("WEB-12"));
  const crumbs = partIn(page, "top-bar.crumb");
  await expect(crumbs.last()).toHaveText("WEB-12");
  await expect(crumbs.first()).toBeVisible();
});
