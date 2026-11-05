import { t } from "@bugaboo/messages";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { issueAddress } from "../src/shared/paths.ts";

// An open issue on a phone (M5-F172, M3-F178, UX §3.3): the bar of the app is hidden, the bar of the
// issue (Status, Comment, Timer, More) is the only bar, so one More shows, and Back in the top bar
// leads out.

test.use({ preparation: { run: prepareWeb } });

test("ux_01 on a phone an open issue shows one bar and one more button", async ({
  page,
  server,
  hasTouch,
}) => {
  test.skip(!hasTouch, "The bar of an issue is the bar of a phone.");
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Printer jams");
  await admin.dispose();
  await signIn(page, ada);
  const appBar = page.getByRole("navigation", { name: t("shell.menu") });
  await page.goto("/");
  await expect(appBar).toBeVisible();
  await page.goto(issueAddress(issue.key));
  const issueBar = page.getByRole("toolbar", { name: t("issues.bar") });
  await expect(issueBar).toBeVisible();
  await expect(appBar).toHaveCount(0);
  await expect(page.getByRole("button", { name: t("issues.more"), exact: true })).toHaveCount(1);
  await expect(issueBar.getByRole("button")).toHaveText([
    t("issue-field.status"),
    t("issues.comment"),
    t("time.timer.bar"),
    t("issues.more"),
  ]);
  await expect(page.getByRole("button", { name: t("shell.back"), exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
  // Back leads out, and the bar of the app is there again.
  await page.getByRole("button", { name: t("shell.back"), exact: true }).click();
  await expect(appBar).toBeVisible();
});
