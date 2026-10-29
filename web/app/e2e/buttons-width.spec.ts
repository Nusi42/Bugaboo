import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O6): on a computer a main button is as wide as its words, wherever it stands: a form
// does not stretch its button over the width of the card (the passkey form of the security page, the sign-in
// rules, the security of the organization). On a phone the buttons of a form may be as wide as the screen.

test.use({ sample: true });

/** The widest main button of the page, as a share of the element it stands in. */
function widestShare(area: Locator): Promise<number> {
  return area.evaluate((node) => {
    let widest = 0;
    for (const button of node.querySelectorAll("[data-part='button'][data-variant~='primary']")) {
      const column = button.parentElement;
      if (column !== null) {
        widest = Math.max(
          widest,
          button.getBoundingClientRect().width / column.getBoundingClientRect().width,
        );
      }
    }
    return widest;
  });
}

test("ux_7_4 on a computer a main button is as wide as its words", async ({ page }) => {
  test.skip(layoutOf(page) !== "computer", "a phone has buttons as wide as the screen");
  await signIn(page, sampleAdministrator());
  await inOrder(
    [paths.handle, paths.security, paths.signInRules, paths.organizationSecurity].map(
      (address) => async () => {
        await page.goto(address);
        await expect(
          page.locator("main [data-part='button'][data-variant~='primary']").first(),
        ).toBeVisible();
        expect(await widestShare(page.locator("main")), `${address}: a main button`).toBeLessThan(
          0.6,
        );
      },
    ),
  );
});

/** How wide the main buttons of a page are, in pixels, each rounded. */
function widthsOfMainButtons(area: Locator): Promise<readonly number[]> {
  return area.evaluate((node) => {
    const widths: number[] = [];
    for (const button of node.querySelectorAll("[data-part='button'][data-variant~='primary']")) {
      widths.push(Math.round(button.getBoundingClientRect().width));
    }
    return widths;
  });
}

test("ux_7_4 on a phone the main buttons of the security page have one width", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "a computer has buttons as wide as their words");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.security);
  await expect(
    page.locator("main [data-part='button'][data-variant~='primary']").first(),
  ).toBeVisible();
  const widths = await widthsOfMainButtons(page.locator("main"));
  expect(widths.length).toBeGreaterThan(1);
  expect([...new Set(widths)], "the widths of the main buttons").toHaveLength(1);
});
