import { t } from "@bugaboo/messages";
import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O3): the list of people to put into a group shows whole rows: no row is cut by the
// edge of the list where it starts to scroll.

test.use({ sample: true });

/** How many rows are cut by the lower edge of a list that scrolls, as it stands at its start. */
function rowsCutAtTheEnd(list: Locator, rows = ":scope > *"): Promise<number> {
  return list.evaluate((node, selector) => {
    const edge = node.getBoundingClientRect().top + node.clientTop + node.clientHeight;
    let cut = 0;
    for (const row of node.querySelectorAll(selector)) {
      const box = row.getBoundingClientRect();
      if (box.top < edge && box.bottom > edge + 0.5) {
        cut += 1;
      }
    }
    return cut;
  }, rows);
}

test("ux_5 the list of people to put in shows whole rows at its start", async ({ page }) => {
  test.skip(layoutOf(page) !== "computer", "the panel is for a computer");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.groups);
  await page.getByRole("button", { name: "Backend", exact: true }).click();
  const panel = page.getByRole("complementary", { name: "Backend" });
  const list = partIn(panel, "picker.options").last();
  await expect(list).toBeVisible();
  expect(await rowsCutAtTheEnd(list), "rows cut by the lower edge of the list").toBe(0);
});

test("ux_5 the list of the people shows whole rows at its start and when it is scrolled", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.people);
  const list = page.getByRole("listbox", { name: t("people.list") });
  await expect(list.getByRole("option").first()).toBeVisible();
  expect(await rowsCutAtTheEnd(list, "[role='option']"), "rows cut at the start").toBe(0);
});
