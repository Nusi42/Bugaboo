import { type Locator } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// M5 exit, the final pass: the explanation of a setting that is a switch or a group of choices is the same hint as
// under a box typed in (size, colour, wrap), and "Default value" stands at the same distance under it.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await admin.dispose();
    },
  },
});

/** Half a pixel either way: boxes are measured in fractions. */
const ROUNDING = 0.5;

/** The look of a hint that has to be the same for every kind of setting. */
function lookOf(hint: Locator) {
  return hint.evaluate((node) => {
    const style = getComputedStyle(node);
    return { size: style.fontSize, color: style.color, wrap: style.textWrap };
  });
}

/** How far the note of where the value comes from stands under the hint before it, in the box of one setting. */
async function gapToSource(setting: Locator, hint: Locator): Promise<number> {
  const above = await boxOf(hint);
  const below = await boxOf(partIn(setting, "settings.source"));
  return below.y - (above.y + above.height);
}

test("cfg_06 the hint of a switch is the hint of a box, at the same distance from its note", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.signInRules);
  const boxes = partIn(page, "box");
  const typed = boxes.filter({ has: partIn(page, "field.description") }).first();
  const switched = boxes.filter({ has: page.getByRole("switch") }).first();
  const typedHint = partIn(typed, "field.description");
  const switchHint = partIn(switched, "settings.help");
  await expect(switchHint).toBeVisible();
  expect(await lookOf(switchHint)).toEqual(await lookOf(typedHint));
  const typedGap = await gapToSource(typed, typedHint);
  expect(Math.abs((await gapToSource(switched, switchHint)) - typedGap)).toBeLessThanOrEqual(
    ROUNDING,
  );
});
