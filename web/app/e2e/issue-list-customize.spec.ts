import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareLook, rowOf } from "./support/list-look.ts";
import { boxOf } from "./support/windows.ts";

// *Customize* (UX §4.2): the panel stands beside the list where there is room for both, and under
// it where there is not.

test.use({ preparation: { run: prepareLook } });

test("ux_4_2 Customize sits beside the list on a computer window of 1280", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone has no Customize");
  await signIn(page, ada);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: t("issues.customize") });
  const [list, side] = [await boxOf(page.locator("[data-part='list-main']")), await boxOf(panel)];
  expect(side.x).toBeGreaterThanOrEqual(list.x + list.width);
  // Every default column stays in view beside Customize: the list does not scroll sideways, and
  // the head of each column is inside the box of the list (UX §4.2, R5, R9).
  const frame = page.locator("[data-part='list-frame']");
  expect(await frame.evaluate((box) => box.scrollWidth - box.clientWidth)).toBe(0);
  const main = await boxOf(page.locator("[data-part='list-main']"));
  const heads = await Promise.all(
    [t("issue-field.priority"), t("issue-field.assigned-to"), t("issue-field.changed")].map(
      (column) => boxOf(page.getByRole("columnheader", { name: column, exact: true })),
    ),
  );
  for (const head of heads) {
    expect(head.x + head.width).toBeLessThanOrEqual(main.x + main.width + 0.5);
  }
});
