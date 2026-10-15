import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb, webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";

// The list of issues for a screen reader and a finger (UX §4.2, §8, M3 WP-13): the type of an
// issue is named, the title is a target a finger can hit, and ticking an issue is told.

test.use({ preparation: { run: prepareWeb } });

test("ux_08 a row names the type of its issue, and a tick is told", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const row = page.getByRole("listitem").filter({ hasText: webIssues.passkey });
  await expect(row.getByRole("img")).toHaveAccessibleName(/\S/u);
  // A person presses the box they see, not the input behind it.
  await row.locator("label").click();
  await expect(
    page.getByRole("status").filter({ hasText: t("issues.selected", { count: 1 }) }),
  ).toHaveCount(1);
});

test("ux_08 on a phone the title of an issue is as high as a finger", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has to be pressed with a finger");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const link = page.getByRole("link", { name: webIssues.passkey });
  const box = await link.boundingBox();
  expect(box?.height).toBeGreaterThanOrEqual(44);
});
