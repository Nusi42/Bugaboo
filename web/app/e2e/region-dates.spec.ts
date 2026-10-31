import { t } from "@bugaboo/messages";
import { type Browser, type Locator, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { createOwnField, setOwnValue } from "./support/fields.ts";
import { expect, test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { asBen, follow, hearOfFieldChanges, openNotifications } from "./support/notifications.ts";
import { chooseTimeZone } from "./support/region.ts";

// One way to write a date and a time of day for a person (UX-07, CFG-04, M5-F137): with the date
// day first, the clock of 12 hours and the zone of Berlin under *Region*, the same moment (a value
// of the field *Starts*) is written the same way in a column of the list, in the chip of a search
// for its day, and in the notification of its change.

test.use({ preparation: { run: prepareWeb } });

/** The moment Ben writes into *Starts*: 9:30 in the morning in Berlin, in summer. */
const MOMENT = "2026-10-05T07:30:00Z";
/** How Ada reads it, with plain spaces. */
const WRITTEN = "05.10.2026, 9:30 AM";
/** How Ada reads its day. */
const DAY = "05.10.2026";
/** A text that starts with a date written day first. */
const DAY_FIRST = /\d{2}\.\d{2}\.\d{4}/u;

/** Text with every kind of space as a plain one: a date never breaks, so its spaces hold together. */
function plain(text: string | null): string {
  return (text ?? "").replaceAll(/\s/gu, " ");
}

async function choose(radio: Locator): Promise<void> {
  await radio.click();
  await expect(radio).toBeChecked();
}

/** Chooses Berlin, the date day first and the clock of 12 hours on *Region*. */
async function chooseBerlinDayFirstOn12Hours(page: Page): Promise<void> {
  await chooseTimeZone(page, "Europe/Berlin");
  await choose(
    page.getByRole("radio", { name: t("setting.region.date-format.day-month-year"), exact: true }),
  );
  await choose(
    page.getByRole("radio", { name: t("setting.region.time-of-day.twelve-hours"), exact: true }),
  );
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
}

/**
 * Ada makes the field *Starts*, chooses her region and follows WEB-1 with its fields; Ben writes the moment into
 * *Starts* of WEB-1, which Ada is told about.
 */
async function benWritesTheMoment(
  page: Page,
  browser: Browser,
  server: Parameters<typeof apiOf>[1],
): Promise<void> {
  await signIn(page, ada);
  const own = apiOf(page, server);
  const starts = (await createOwnField(own, "Starts", "date-time")).id ?? "";
  await chooseBerlinDayFirstOn12Hours(page);
  await follow(own, "WEB-1");
  await hearOfFieldChanges(own);
  await setOwnValue(apiOf(await asBen(browser, server), server), "WEB-1", starts, MOMENT);
}

test("ux_07_a_date_and_a_time_of_day_are_written_one_way_for_a_person_everywhere", async ({
  page,
  browser,
  server,
}) => {
  test.skip(test.info().project.name !== "desktop", "a computer shows the column of a moment");
  await benWritesTheMoment(page, browser, server);

  // The column *Starts* of the list.
  await page.goto(`${projectIssuesAddress("WEB")}?cols=id,title,starts`);
  const cell = page.getByText(DAY_FIRST).first();
  await expect(cell).toBeVisible();
  expect(plain(await cell.textContent())).toBe(WRITTEN);

  // The chip of a search for that day writes the same day.
  await page.goto(`${projectIssuesAddress("WEB")}?s=${encodeURIComponent("starts is 2026-10-05")}`);
  const chip = page.getByRole("button", { name: DAY_FIRST }).first();
  await expect(chip).toBeVisible();
  expect(plain(await chip.textContent())).toContain(DAY);

  // The notification of the change writes the same moment.
  await openNotifications(page);
  const said = page.getByText(/Starts/u).first();
  await expect(said).toBeVisible();
  expect(plain(await said.textContent())).toContain(WRITTEN);
});
