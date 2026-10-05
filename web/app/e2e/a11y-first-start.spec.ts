import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";

// The first start has its own server: it only exists while nobody has an account. The other
// addresses are in a11y.spec.ts (TEST-A11Y).

test(`ux_04 ${paths.setup} has no accessibility problems in the four themes`, async ({ page }) => {
  await page.goto(paths.setup);
  await expect(page.getByRole("heading", { level: 1, name: t("first-run.title") })).toBeVisible();
  await expectGoodInEveryTheme(page);
});
