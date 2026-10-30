import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareLook, rowOf } from "./support/list-look.ts";
import { boxOf, roomUnderContent, shadowsOf } from "./support/windows.ts";

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
  const [list, side] = [await boxOf(page.locator("[data-part='list-column']")), await boxOf(panel)];
  expect(side.x).toBeGreaterThanOrEqual(list.x + list.width);
  // The panel is level with the head of the list (L13).
  const head = await boxOf(page.locator("[data-part='list-head']"));
  expect(Math.abs(side.y - head.y)).toBeLessThanOrEqual(2);
  // Beside Customize the title keeps its least width of 160 and the columns of fields that do not
  // fit are dropped from the right: the list does not scroll sideways and every head is in the box
  // of the list (UX §4.2, R5, R9).
  const frame = page.locator("[data-part='list-frame']");
  expect(await frame.evaluate((box) => box.scrollWidth - box.clientWidth)).toBe(0);
  const main = await boxOf(page.locator("[data-part='list-column']"));
  const title = await boxOf(
    page.getByRole("columnheader", { name: t("issue-field.title"), exact: true }),
  );
  expect(title.width).toBeGreaterThanOrEqual(160);
  const priority = await boxOf(
    page.getByRole("columnheader", { name: t("issue-field.priority"), exact: true }),
  );
  expect(priority.x + priority.width).toBeLessThanOrEqual(main.x + main.width + 0.5);
});

test("ux_4_2 Customize under the list is as wide as the list, in more than one column (L15)", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone has no Customize");
  await signIn(page, ada);
  await page.setViewportSize({ width: 900, height: 1000 });
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: t("issues.customize") });
  const [list, under] = [
    await boxOf(page.locator("[data-part='list-column']")),
    await boxOf(panel),
  ];
  expect(under.y).toBeGreaterThanOrEqual(list.y + list.height);
  expect(Math.abs(under.width - list.width)).toBeLessThanOrEqual(2);
  const columns = await panel
    .locator("[data-part='list-panel.items']")
    .evaluate((box) => getComputedStyle(box).gridTemplateColumns.split(" ").length);
  expect(columns).toBeGreaterThanOrEqual(2);
  // The panel is as high as its columns, with no empty room under them; the list takes the rest (review of package A).
  expect(await roomUnderContent(panel)).toBeLessThanOrEqual(1);
});

test("ux_4_2 Customize beside the list fits the room the list has: its columns scroll inside it (review of the full-height list)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer has Customize beside the list");
  await signIn(page, ada);
  // A low window, so the columns of Customize do not all fit.
  await page.setViewportSize({ width: 1280, height: 600 });
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: t("issues.customize") });
  const [side, rows] = await Promise.all([
    boxOf(panel),
    boxOf(page.locator("[data-part='list-scroll']")),
  ]);
  // In this low window the rows fill the rest of it: the panel, as high as its line, ends with the box of the rows,
  // inside the card and the window; what does not fit scrolls under a shadow.
  expect(Math.abs(side.y + side.height - (rows.y + rows.height))).toBeLessThanOrEqual(2);
  await expect.poll(() => shadowsOf(panel)).toEqual({ start: 0, end: 1 });
});
