import { t } from "@bugaboo/messages";

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

test("ux_05 the keys of Keys now and Usual keys start on one edge", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.shortcuts);
  await page.getByRole("button", { name: t("shortcut.go-home"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: t("shortcut.go-home"), exact: true });
  const rows = partIn(panel, "shortcut-editor.row");
  const now = await boxOf(
    rows
      .filter({ hasText: t("shortcuts.now") })
      .locator("kbd")
      .first(),
  );
  const usual = await boxOf(
    rows
      .filter({ hasText: t("shortcuts.usual") })
      .locator("kbd")
      .first(),
  );
  expect(now.x).toBe(usual.x);
});
