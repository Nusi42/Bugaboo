import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { tickBox, webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { chooseSpacing, listFrame, prepareLook, rowOf } from "./support/list-look.ts";
import { boxOf } from "./support/windows.ts";

// The list that is wider than its box (UX §4.2): the head scrolls with the rows; and the person's
// Compact spacing keeps a target a finger can press on touch (UX §7.4).

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

test("ux_4_2 the head scrolls sideways with the rows", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone has cards and no head");
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?group=-&widths=title:1600`);
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  const frame = listFrame(page);
  // The list is wider than its box and scrolls sideways; the head is in the box that scrolls, so it
  // moves with the rows: the same left edge for a cell of the head and a cell of a row.
  await frame.evaluate((box) => {
    box.scrollLeft = 300;
  });
  await expect.poll(() => frame.evaluate((box) => Math.round(box.scrollLeft))).toBeGreaterThan(0);
  const titleOf = (where: string) =>
    page.locator(`${where} [data-part='list-row.cell'][data-variant='title']`).first();
  const [headCell, rowCell] = [
    await boxOf(titleOf("[data-part='list-head']")),
    await boxOf(titleOf("[data-part='list-scroll']")),
  ];
  expect(Math.abs(headCell.x - rowCell.x)).toBeLessThanOrEqual(1);
});

test("ux_7_4 the tick box of a compact list keeps a target of 44 on touch", async ({
  page,
  server,
}) => {
  test.skip(layoutOf(page) !== "phone", "touch is the phone project");
  await chooseSpacing(server, "compact");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await expect(listFrame(page)).toHaveAttribute("data-variant", /compact/u);
  const box = await boxOf(tickBox(page, "WEB-1").locator("xpath=ancestor::label"));
  expect(box.height).toBeGreaterThanOrEqual(44);
  await chooseSpacing(server, "comfortable");
});

test("ux_4_2 a list that was wide and becomes a phone's cards does not scroll sideways", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "it starts as a computer and becomes a phone");
  await signIn(page, ada);
  await page.setViewportSize({ width: 1024, height: 800 });
  await page.goto(`${projectIssuesAddress("WEB")}?group=-&widths=title:1600`);
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  const frame = listFrame(page);
  expect(await frame.evaluate((box) => box.scrollWidth - box.clientWidth)).toBeGreaterThan(0);
  // The same page, without a reload, on a phone: the head is gone and so is the width it gave.
  await page.setViewportSize({ width: 390, height: 800 });
  await expect.poll(() => frame.evaluate((box) => box.scrollWidth - box.clientWidth)).toBe(0);
});
