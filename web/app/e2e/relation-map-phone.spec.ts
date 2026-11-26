import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { cardOf, mapList, mapped, openMap, prepareRelationMap } from "./support/relation-map.ts";

// The relation map on a phone (ISS-16, UX §3.3, §4.15): the list form comes first, with *Show as
// picture*; the picture keeps the chain of a pressed issue in a sheet.

test.use({ preparation: { run: prepareRelationMap } });

test("iss_16_the_phone_starts_in_the_list_form", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await expect(mapList(page)).toBeVisible();
  await expect(page.locator('[data-part="map-card"]')).toHaveCount(0);
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("relation-map.show-as-picture"), exact: true }).click();
  await expect(cardOf(page, mapped.start)).toBeVisible();
  await expect(
    page.getByRole("button", { name: t("relation-map.show-as-list"), exact: true }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("iss_16_switching_between_list_and_picture_keeps_the_zoom", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await page.getByRole("button", { name: t("relation-map.show-as-picture"), exact: true }).click();
  const zoom = page.getByLabel(t("relation-map.zoom"), { exact: true });
  const chosen = t("relation-map.zoom-level", { percent: 125 });
  await zoom.selectOption({ label: chosen });
  await page.getByRole("button", { name: t("relation-map.show-as-list"), exact: true }).click();
  await page.getByRole("button", { name: t("relation-map.show-as-picture"), exact: true }).click();
  await expect(zoom.locator("option:checked")).toHaveText(chosen);
});
