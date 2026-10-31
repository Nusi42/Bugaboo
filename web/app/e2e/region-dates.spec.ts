import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { asBen, comment, follow, openNotifications } from "./support/notifications.ts";

// One way to write a date and a time of day for a person (UX-07, CFG-04, M5-F137): with the date
// day first and the clock of 12 hours under *Region*, a column of the list, the chip of a search
// and a notification write the moment the same way.

test.use({ preparation: { run: prepareWeb } });

/** A date day first, a comma, and a time on the clock of 12 hours: `31.10.2026, 2:30 PM`. */
const MOMENT_DAY_FIRST_ON_12_HOURS = /^(?<day>\d{2}\.\d{2}\.\d{4}),\s\d{1,2}:\d{2}\s[AP]M$/u;

/** The date of a moment written day first; a text that is none is a failure of the test. */
function dayOf(text: string | null): string {
  const day = MOMENT_DAY_FIRST_ON_12_HOURS.exec(text ?? "")?.groups?.day;
  if (day === undefined) {
    throw new Error(`${String(text)} is no moment written day first on 12 hours.`);
  }
  return day;
}

/** A date written day first (`31.10.2026`) as a search writes it (`2026-10-31`). */
function searchDayOf(day: string): string {
  return day.split(".").toReversed().join("-");
}

async function choose(radio: Locator): Promise<void> {
  await radio.click();
  await expect(radio).toBeChecked();
}

/** Chooses the date day first and the clock of 12 hours on *Region*. */
async function chooseDayFirstOn12Hours(page: Page): Promise<void> {
  await page.goto(paths.region);
  await choose(
    page.getByRole("radio", { name: t("setting.region.date-format.day-month-year"), exact: true }),
  );
  await choose(
    page.getByRole("radio", { name: t("setting.region.time-of-day.twelve-hours"), exact: true }),
  );
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
}

test("ux_07_a_date_and_a_time_of_day_are_written_one_way_for_a_person_everywhere", async ({
  page,
  browser,
  server,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer shows the column of a moment");
  await signIn(page, ada);
  await chooseDayFirstOn12Hours(page);
  await follow(apiOf(page, server), "WEB-1");
  const ben = await asBen(browser, server);
  await comment(apiOf(ben, server), "WEB-1", "A note at a moment Ada sees in her own way.");

  // The column *Created* of the list.
  await page.goto(`${projectIssuesAddress("WEB")}?cols=id,title,created`);
  const cell = page.getByText(MOMENT_DAY_FIRST_ON_12_HOURS).first();
  await expect(cell).toBeVisible();
  const day = dayOf(await cell.textContent());

  // The chip of a search for that day writes the same date.
  await page.goto(
    `${projectIssuesAddress("WEB")}?s=${encodeURIComponent(`created is ${searchDayOf(day)}`)}`,
  );
  await expect(page.getByRole("button", { name: new RegExp(day, "u") }).first()).toBeVisible();

  // A notification writes its moment the same way.
  await openNotifications(page);
  await expect(
    page.locator('[data-part="history.when"]').getByText(MOMENT_DAY_FIRST_ON_12_HOURS).first(),
  ).toBeVisible();
});
