import { type Locator } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";

// M5 exit, the final pass: the explanation of a setting that is a switch or a group of choices is the same hint as
// under a box typed in (size, colour, wrap). The line that says where a value comes from is left out for the standard (M6-F177), so there
// is no distance to it to compare.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await admin.dispose();
    },
  },
});

/** The look of a hint that has to be the same for every kind of setting. */
function lookOf(hint: Locator) {
  return hint.evaluate((node) => {
    const style = getComputedStyle(node);
    return { size: style.fontSize, color: style.color, wrap: style.textWrap };
  });
}

test("cfg_06 the hint of a switch is the hint of a box", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.signInRules);
  const boxes = partIn(page, "box");
  const typed = boxes.filter({ has: partIn(page, "field.description") }).first();
  const switched = boxes.filter({ has: page.getByRole("switch") }).first();
  const typedHint = partIn(typed, "field.description");
  const switchHint = partIn(switched, "settings.help");
  await expect(switchHint).toBeVisible();
  expect(await lookOf(switchHint)).toEqual(await lookOf(typedHint));
});
