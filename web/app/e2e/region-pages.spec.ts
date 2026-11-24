import { t } from "@bugaboo/messages";

import { formatCalendarDay } from "../src/shared/format.ts";
import { paths } from "../src/shared/paths.ts";
import { deviceRegion } from "../src/shared/region.ts";
import { bob, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { aboutOf } from "./support/about-words.ts";
import { FILE_CREATED_MS, preparePrivacy } from "./support/privacy.ts";
import { chooseDateFormat } from "./support/region.ts";

// *About* and *My data* write their dates from the person's *Region*, as every other screen does (UX-07, TXT-DATES).

test.use({ preparation: { run: preparePrivacy } });

/** A date written day first, as `24.11.2026`. */
const DAY_FIRST = /\d{2}\.\d{2}\.\d{4}/u;

test("ux_07 About and My data write their dates the way the person chose under Region", async ({
  page,
  server,
}) => {
  const own = await signInThroughApi(server, bob);
  await chooseDateFormat(own, "day-month-year");
  await own.dispose();
  await signIn(page, bob);
  const released = (await aboutOf(server)).support_period.released;
  await page.goto(paths.about);
  const dayFirst = { ...deviceRegion, dateFormat: "day-month-year" } as const;
  await expect(
    page.getByText(formatCalendarDay(released, dayFirst), { exact: true }),
  ).toBeVisible();
  expect(formatCalendarDay(released, dayFirst)).toMatch(DAY_FIRST);
  await page.goto(paths.myData);
  const download = page.getByRole("region", { name: t("my-data.download"), exact: true });
  await download.getByRole("button", { name: t("my-data.download"), exact: true }).click();
  await expect(
    download.getByRole("button", { name: t("my-data.download-file"), exact: true }),
  ).toBeVisible({ timeout: FILE_CREATED_MS });
  await expect(download.getByText(DAY_FIRST).first()).toBeVisible();
});
