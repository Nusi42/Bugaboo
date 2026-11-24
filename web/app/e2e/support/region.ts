import { type SettingChoice, settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { type ApiSession } from "./api.ts";

/** Chooses a time zone on *Region* and waits until the server has it. */
export async function chooseTimeZone(page: Page, zone: string): Promise<void> {
  await page.goto(paths.region);
  await page
    .getByRole("combobox", { name: t("setting.region.time-zone.name"), exact: true })
    .selectOption(zone);
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
}

/** Chooses how the person writes dates, through the API (the screen for it is tested with *Region*). */
export async function chooseDateFormat(
  session: ApiSession,
  format: SettingChoice<"regionDateFormat">,
): Promise<void> {
  await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.regionDateFormat } },
    body: { value: format },
  });
}
