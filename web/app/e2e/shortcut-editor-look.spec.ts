import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin, forgetChoices } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// The editor of a shortcut (the final check of M4): the keys of *Keys now* and of *Usual keys* start
// on one edge, so the two rows read as one table.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test.beforeEach(async ({ server }) => {
  await forgetChoices(server);
});

/** The two rows of the editor, opened for *Go home*. */
async function openRows(page: Page) {
  await signIn(page, ada);
  await page.goto(paths.shortcuts);
  await page.getByRole("button", { name: t("shortcut.go-home"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: t("shortcut.go-home"), exact: true });
  const rows = partIn(panel, "shortcut-editor.row");
  return {
    labels: partIn(panel, "shortcut-editor.label"),
    now: rows
      .filter({ hasText: t("shortcuts.now") })
      .locator("kbd")
      .first(),
    usual: rows
      .filter({ hasText: t("shortcuts.usual") })
      .locator("kbd")
      .first(),
  };
}

test("ux_05 the keys of Keys now and Usual keys start on one edge", async ({ page }) => {
  const { now, usual } = await openRows(page);
  expect((await boxOf(now)).x).toBeCloseTo((await boxOf(usual)).x, 1);
});

for (const size of ["32px", "9px"]) {
  test(`ux_05 they still do when the labels are set in ${size} type`, async ({ page }) => {
    const { labels, now, usual } = await openRows(page);
    await labels.evaluateAll((nodes: readonly HTMLElement[], value) => {
      for (const node of nodes) {
        node.style.fontSize = value;
      }
    }, size);
    expect((await boxOf(now)).x).toBeCloseTo((await boxOf(usual)).x, 1);
  });
}
