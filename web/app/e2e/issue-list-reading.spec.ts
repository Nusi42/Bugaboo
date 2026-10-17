import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb, webIssues } from "./support/issues.ts";
import { layoutOf, onThisLayout } from "./support/layout.ts";
import { rowOf } from "./support/list-look.ts";

// The list of issues for a screen reader and a finger (UX §4.2, §8, M3 WP-13): the type of an
// issue is named, the title is a target a finger can hit, and ticking an issue is told.

test.use({ preparation: { run: prepareWeb } });

test("ux_08 a row names the type of its issue, and a tick is told", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const row = rowOf(page, webIssues.passkey);
  // The type is named, not only drawn (the scenarios make their issues of the type Task); a phone row has no type.
  const task = row.getByRole("img", { name: "Task", exact: true });
  const typeIs = (count: number) => async () => {
    await expect(task).toHaveCount(count);
  };
  await onThisLayout(page, { computer: typeIs(1), tablet: typeIs(1), phone: typeIs(0) });
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
  // The title link reaches over the whole row (R8): the row is the target a finger hits.
  const box = await rowOf(page, webIssues.passkey).boundingBox();
  expect(box?.height).toBeGreaterThanOrEqual(44);
});
