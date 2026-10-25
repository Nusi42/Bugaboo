import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";

/** Chooses a time zone on *Region* and waits until the server has it. */
export async function chooseTimeZone(page: Page, zone: string): Promise<void> {
  await page.goto(paths.region);
  await page
    .getByRole("combobox", { name: t("setting.region.time-zone.name"), exact: true })
    .selectOption(zone);
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
}
