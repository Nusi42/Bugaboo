import { type Page } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (the one-title rule of L): every screen of a phone has exactly one visible title: the
// title of the top bar where there is one, the heading of the card where there is none (the sign-in).

test.use({ sample: true });

/** How many titles are drawn: the headings of level 1 that are seen, and the title of the top bar. */
function titlesShown(page: Page): Promise<number> {
  return page.evaluate(() => {
    let shown = 0;
    for (const heading of document.querySelectorAll("h1")) {
      if (heading.getBoundingClientRect().width > 2) {
        shown += 1;
      }
    }
    const bar = document.querySelector("[data-part='top-bar.title']");
    return bar !== null && bar.getBoundingClientRect().width > 2 ? shown + 1 : shown;
  });
}

test("ux_3 a phone shows one title on the sign-in, and on every screen after it", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a computer shows the title in the page");
  await page.goto(paths.signIn);
  await expect(page.getByRole("heading", { level: 1 }).first()).toBeVisible();
  expect(await titlesShown(page), paths.signIn).toBe(1);
  await signIn(page, sampleAdministrator());
  await inOrder(
    [
      paths.home,
      paths.mySettings,
      paths.appearance,
      paths.organizationSettings,
      paths.people,
      paths.security,
    ].map((address) => async () => {
      await page.goto(address);
      await expect(page.locator("[data-part='top-bar.title']")).toBeVisible();
      expect(await titlesShown(page), address).toBe(1);
    }),
  );
});
