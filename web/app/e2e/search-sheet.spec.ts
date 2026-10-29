import { t } from "@bugaboo/messages";

import { createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { box, boxOf, openBar, openSearch } from "./support/search-bar.ts";

// The sheet of the search bar on a phone and the offers of Advanced search (exit polish of M5, S3,
// S4, S14): the offers stand in the flow, and the foot of the sheet is always in the screen.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      await admin.dispose();
    },
  },
});

test("qry_03 the offers never push the foot of the sheet off the screen or cover the page", async ({
  page,
}) => {
  await openSearch(page, "assignedTo is me");
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill("prio is high and ");
  const offers = page.getByRole("listbox");
  await expect(offers).toBeVisible();
  await expect(offers).not.toHaveCSS("position", "absolute");
  const offered = await boxOf(offers);
  const field = await boxOf(box(page));
  // The offers stand under the box in the flow of the page, they do not float over what follows it.
  expect(offered.y).toBeGreaterThanOrEqual(field.y + field.height);
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: async () => {
      const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
      const button = sheet.getByRole("button", { name: t("search.bar.sheet-done"), exact: true });
      await expect(button).toBeInViewport({ ratio: 1 });
      const [sheetBox, buttonBox] = await Promise.all([boxOf(sheet), boxOf(button)]);
      // The offers start above the foot (the body of the sheet scrolls to the rest), and the foot
      // stays where it is, in the screen.
      expect(offered.y).toBeLessThan(buttonBox.y);
      expect(buttonBox.y + buttonBox.height).toBeLessThanOrEqual(sheetBox.y + sheetBox.height);
    },
  });
});

test("qry_06 on a phone Save search stands with Done in the foot of the sheet", async ({
  page,
}) => {
  await openSearch(page, "priority is High");
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: async () => {
      await openBar(page);
      const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
      const save = sheet.getByRole("button", { name: t("search.save"), exact: true });
      const done = sheet.getByRole("button", { name: t("search.bar.sheet-done"), exact: true });
      // The sheet slides in from the bottom: the boxes are read together, until it stands still.
      await expect
        .poll(async () => {
          const [sheetBox, saveBox, doneBox] = await Promise.all([
            boxOf(sheet),
            boxOf(save),
            boxOf(done),
          ]);
          // The foot of a window has one pattern (package D): Done as wide as the foot, Save search
          // under it, and the whole foot inside the sheet.
          // The least of the three gaps (aligned, under, inside) is not negative when all hold.
          return Math.min(
            1 - Math.abs(saveBox.x - doneBox.x),
            saveBox.y - (doneBox.y + doneBox.height) + 1,
            sheetBox.y + sheetBox.height - (saveBox.y + saveBox.height),
          );
        })
        .toBeGreaterThanOrEqual(0);
    },
  });
});
