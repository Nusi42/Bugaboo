import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { tickBox, webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { chooseSpacing, listFrame, prepareLook, rowOf } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// Ticking on a phone (UX §4.2, §7.4; M5 exit): the bar of the ticked issues stands above the
// bottom navigation, and the tick box is a target of 44 px also where the person chose Compact.

test.use({ preparation: { run: prepareLook } });

test("ux_4_2 on a phone the bar of the ticked issues is in view, above the bottom navigation", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the bar above the navigation");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await tickBox(page, "WEB-1").check({ force: true });
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

test("ux_7_4 on a phone the tick box is at least 44 px, also in Compact", async ({
  page,
  server,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the cards");
  await chooseSpacing(server, "compact");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await expect(listFrame(page)).toHaveAttribute("data-variant", /compact/u);
  const box = await boxOf(partIn(rowOf(page, webIssues.login), "checkbox"));
  expect(box.width).toBeGreaterThanOrEqual(44);
  expect(box.height).toBeGreaterThanOrEqual(44);
  await chooseSpacing(server, "comfortable");
});
