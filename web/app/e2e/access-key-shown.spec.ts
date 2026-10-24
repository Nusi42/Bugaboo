import { t } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { lineLengths } from "./support/line-lengths.ts";
import { partIn } from "./support/parts.ts";

// The key that is shown once (the final check of M4): its lines are even, so that two letters never
// stand alone on the last line; and in the list the words *Starts with* are plain words, with only
// the start of the key in the code font.

/** The lines of the key differ by no more than this many characters: two letters never stand alone. */
const MOST_UNEVEN = 3;

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test("auth_07 the key that is shown has even lines, and only its start is in the code font", async ({
  page,
}) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.access-keys"));
  await page.getByRole("button", { name: t("access-keys.create"), exact: true }).click();
  const form = page.getByRole("region", { name: t("access-keys.create"), exact: true });
  await form.getByLabel(t("access-keys.name")).fill("Backup script");
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await form.getByRole("button", { name: t("access-keys.submit") }).click();
  const shown = page.getByRole("region", { name: t("access-keys.shown-title") });
  const lines = await lineLengths(shown.locator("code"));
  expect(Math.max(...lines) - Math.min(...lines)).toBeLessThanOrEqual(MOST_UNEVEN);
  await shown.getByRole("button", { name: t("access-keys.done") }).click();
  // The words before the start of the key are not in the code font.
  const row = page.getByRole("listitem").filter({ hasText: "Backup script" });
  const code = await partIn(row, "access-keys.prefix").evaluate(
    (node) => getComputedStyle(node).fontFamily,
  );
  const words = row.getByText(t("access-keys.row.prefix", { prefix: "" }).trim()).first();
  expect(await words.evaluate((node) => getComputedStyle(node).fontFamily)).not.toBe(code);
});
