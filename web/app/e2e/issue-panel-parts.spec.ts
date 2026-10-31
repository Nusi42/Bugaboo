import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { preparePanel } from "./support/list-panel.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// The parts of the issue in the side panel (UX §4.2, §4.4; M5 exit L2, L3): the head stays on one
// row, and the comment box is quiet.

test.use({ preparation: { run: preparePanel } });

test("ux_4_2 the head of the panel stays on one row: the address, the full page and More (L2)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer has the panel beside the list");
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?open=WEB-1`);
  const panel = page.getByRole("complementary").first();
  await expect(panel.getByRole("button", { name: t("issues.more"), exact: true })).toBeVisible();
  const tops = await Promise.all(
    [
      panel.getByRole("button", { name: t("issues.copy-link-of", { key: "WEB-1" }), exact: true }),
      panel.getByRole("button", { name: t("issues.open-full-page"), exact: true }),
      panel.getByRole("button", { name: t("issues.more"), exact: true }),
    ].map(async (button) => (await boxOf(button)).y),
  );
  expect(Math.max(...tops) - Math.min(...tops)).toBeLessThanOrEqual(2);
});

test("ux_4_2 the comment box in the panel has quiet icon buttons and one footer row with Comment at the end (L3)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer has the panel beside the list");
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?open=WEB-1`);
  const panel = page.getByRole("complementary").first();
  const send = panel.getByRole("button", { name: t("comments.add"), exact: true });
  const files = panel.getByText(t("files.add-to-comment"), { exact: true });
  await expect(send).toBeVisible();
  const [sent, added] = [await boxOf(send), await boxOf(files)];
  expect(Math.abs(sent.y + sent.height / 2 - (added.y + added.height / 2))).toBeLessThanOrEqual(4);
  expect(sent.x).toBeGreaterThan(added.x + added.width);
  // The formatting actions are icons, not words.
  const bold = panel.getByRole("button", { name: t("markdown.bold"), exact: true });
  await expect(bold).toBeVisible();
  expect((await boxOf(bold)).width).toBeLessThanOrEqual(48);
});

test("ux_4_2 beside the panel a filter chip does not wrap inside itself, and Customize stays level with Group by (A3)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer has the panel beside the list");
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?s=statusGroup+is+not+Done&open=WEB-1`);
  const chip = partIn(page, "search-filter").first();
  await expect(chip).toBeVisible();
  // One line: a chip as high as the buttons beside it, not two lines of text.
  expect((await boxOf(chip)).height).toBeLessThanOrEqual(48);
  // And its words are whole: the chip grows to its label and the buttons beside it move below, so no ellipsis cuts the value.
  const words = chip.locator("button").first();
  const cut = await words.evaluate((node) => node.scrollWidth > node.clientWidth);
  expect(cut, "the label of the chip is cut by an ellipsis").toBe(false);
  const customize = await boxOf(page.getByRole("button", { name: t("issues.customize") }));
  const groupBy = await boxOf(page.getByLabel(t("issues.group-by"), { exact: true }));
  expect(
    Math.abs(customize.y + customize.height - (groupBy.y + groupBy.height)),
  ).toBeLessThanOrEqual(12);
  // Spacing is the person's own (My settings › Appearance): it is not above the list.
  await expect(page.getByText(t("appearance.spacing.legend"), { exact: true })).toHaveCount(0);
});
