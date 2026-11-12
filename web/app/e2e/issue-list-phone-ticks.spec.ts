import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareLook, rowOf, tickIssue } from "./support/list-look.ts";
import { boxOf } from "./support/windows.ts";

// Ticking on a phone (UX §4.2, §7.4; M5 exit): the bar of the ticked issues stands above the
// bottom navigation. That the tick box keeps 44 px in Compact is `touch-compact-phone.spec.ts` (UX-02).

test.use({ preparation: { run: prepareLook } });

test("ux_4_2 on a phone the bar of the ticked issues is in view, above the bottom navigation", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the bar above the navigation");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await tickIssue(page, "WEB-1");
  const bar = page.getByRole("region", { name: t("issues.selected-bar"), exact: true });
  await expect(
    bar.getByRole("button", { name: t("many-changes.apply", { count: 1 }) }),
  ).toBeVisible();
  const [box, nav] = [
    await boxOf(bar),
    await boxOf(page.getByRole("navigation", { name: t("shell.menu"), exact: true })),
  ];
  expect(box.y + box.height).toBeLessThanOrEqual(nav.y + 1);
  expect(box.y).toBeGreaterThanOrEqual(0);
});
