import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { preparePanel } from "./support/list-panel.ts";
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
