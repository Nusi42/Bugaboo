import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { boxOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O6): on a computer a main button is as wide as its words, wherever it stands: a form
// does not stretch its button over the width of the card (the sign-in rules, the security of the
// organization). On a phone the buttons of a form may be as wide as the screen.

test.use({ sample: true });

/** How wide a button is, as a share of the width of the element it stands in. */
async function shareOfItsColumn(button: Locator): Promise<number> {
  const [own, column] = [await boxOf(button), await boxOf(button.locator(".."))];
  return own.width / column.width;
}

test("ux_7_4 on a computer a main button is as wide as its words", async ({ page }) => {
  test.skip(layoutOf(page) !== "computer", "a phone has buttons as wide as the screen");
  await signIn(page, sampleAdministrator());
  await inOrder(
    [paths.signInRules, paths.organizationSecurity, paths.security].map((address) => async () => {
      await page.goto(address);
      const main = page.locator("main [data-part='button'][data-variant~='primary']").first();
      await expect(main).toBeVisible();
      expect(await shareOfItsColumn(main), `${address}: a main button`).toBeLessThan(0.6);
    }),
  );
});
