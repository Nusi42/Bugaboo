import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { addDays, formatCalendarDay } from "../src/shared/format.ts";
import { deviceRegion } from "../src/shared/region.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { chooseTimeZone } from "./support/region.ts";

// The day a key runs out (M4, the final check): the day in the box of the form is the day the list
// of keys says afterwards, for a person whose calendar day is not the day in UTC. Two zones are
// used because one of them is on another day than UTC whatever the hour is: 14 hours ahead of UTC
// and 11 hours behind it. A person who chose a time zone of the region has the days counted by that
// zone, the box and the list alike, whatever the clock of the device says (the review of WP-18).

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The day as the list of keys writes it, for a day in the box (`2027-01-23`). */
function written(day: string): string {
  return formatCalendarDay(day, deviceRegion);
}

/** The last Sunday of a month (`month` counts from 0) in UTC, as the day of the month. */
function lastSundayOf(year: number, month: number): number {
  const lastDay = new Date(Date.UTC(year, month + 1, 0));
  return lastDay.getUTCDate() - lastDay.getUTCDay();
}

/**
 * The clock of the page for a day that was typed: 23:30 in Berlin on the eve of the next change of
 * the clocks (the last Saturday of March or of October) after now, so that the default time of a
 * key crosses the change whatever day the test runs on. The eve is at most half a year ahead: the
 * day typed (the default time and a day on) is then still after now and inside the longest time
 * (365 days) that the server counts from its own clock, on every day the test runs.
 */
function eveOfChangeOfClocks(now: number): Date {
  const year = new Date(now).getUTCFullYear();
  for (const candidateYear of [year, year + 1]) {
    // March: 23:30 in Berlin is 22:30 UTC (winter time); October: it is 21:30 UTC (summer time).
    for (const [month, hour] of [
      [2, 22],
      [9, 21],
    ] as const) {
      const eve = new Date(
        Date.UTC(candidateYear, month, lastSundayOf(candidateYear, month) - 1, hour, 30),
      );
      if (eve.getTime() > now) {
        return eve;
      }
    }
  }
  throw new Error("no change of the clocks ahead");
}

/** Makes a key as a person does and returns the day in the box, and whether the day was typed in. */
async function createKeyLikeAPerson(
  page: Page,
  name: string,
  typed: boolean,
  regionZone?: string,
): Promise<string> {
  if (typed) {
    // The server counts the default from its own clock, so only a day that was typed can have the clock of the page.
    await page.clock.install({ time: eveOfChangeOfClocks(Date.now()) });
  }
  await signIn(page, ada);
  if (regionZone !== undefined) {
    await chooseTimeZone(page, regionZone);
  }
  await openFromMenu(page, t("nav.access-keys"));
  await page.getByRole("button", { name: t("access-keys.create"), exact: true }).click();
  const form = page.getByRole("region", { name: t("access-keys.create"), exact: true });
  await form.getByLabel(t("access-keys.name")).fill(name);
  const box = form.getByLabel(t("access-keys.expires"), { exact: true });
  const preset = await box.inputValue();
  // A day typed in is another day than the one filled in: the same text would change nothing.
  const day = typed ? addDays(preset, 1) : preset;
  if (typed) {
    await box.fill(day);
  }
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await form.getByRole("button", { name: t("access-keys.submit") }).click();
  await page
    .getByRole("region", { name: t("access-keys.shown-title") })
    .getByRole("button", { name: t("access-keys.done") })
    .click();
  return day;
}

// Europe/Berlin is the zone with summer time: its days are not all 24 hours long.
for (const timezoneId of ["Pacific/Kiritimati", "Pacific/Pago_Pago", "Europe/Berlin"]) {
  test.describe(`in ${timezoneId}`, () => {
    test.use({ timezoneId });

    for (const [how, typed] of [
      ["the day that is filled in", false],
      ["a day that was typed", true],
    ] as const) {
      test(`auth_07 the key runs out on ${how}`, async ({ page }) => {
        const name = `Key for ${timezoneId}, ${how}`;
        const day = await createKeyLikeAPerson(page, name, typed);
        await expect(
          page
            .getByRole("listitem")
            .filter({ hasText: name })
            .getByText(t("access-keys.row.expires", { when: written(day) }), { exact: true }),
        ).toBeVisible();
      });
    }
  });
}

// The device is 14 hours ahead of UTC, the region of the person is 11 hours behind it: the days
// are the region's days.
test.describe("with a region zone other than the zone of the device", () => {
  test.use({ timezoneId: "Pacific/Kiritimati" });

  for (const [how, typed] of [
    ["the day that is filled in", false],
    ["a day that was typed", true],
  ] as const) {
    test(`auth_07 the key runs out on the day of the region, ${how}`, async ({ page }) => {
      const name = `Key for the region, ${how}`;
      const day = await createKeyLikeAPerson(page, name, typed, "Pacific/Pago_Pago");
      await expect(
        page
          .getByRole("listitem")
          .filter({ hasText: name })
          .getByText(t("access-keys.row.expires", { when: written(day) }), { exact: true }),
      ).toBeVisible();
    });
  }
});

/** Opens the form *Create an access key*, in the region zone given, and returns its box for the day. */
async function openForm(page: Page, regionZone: string) {
  await signIn(page, ada);
  // The person keeps the zone of an earlier test: the days here are counted by the zone chosen now.
  await chooseTimeZone(page, regionZone);
  await openFromMenu(page, t("nav.access-keys"));
  await page.getByRole("button", { name: t("access-keys.create"), exact: true }).click();
  const form = page.getByRole("region", { name: t("access-keys.create"), exact: true });
  return { form, box: form.getByLabel(t("access-keys.expires"), { exact: true }) };
}

test.describe("in Europe/Berlin, with the clock of the page", () => {
  test.use({ timezoneId: "Europe/Berlin" });

  test("auth_07 a form left open past midnight shows the day of now, not the day it was opened", async ({
    page,
  }) => {
    // Signing in takes the time it takes, so the clock starts well before midnight in Berlin (summer
    // time) and is stopped one minute before it once the form shows its day: the page reads its data
    // on timers, which a stopped clock would hold back.
    await page.clock.install({ time: new Date("2027-05-10T21:50:00Z") });
    const { box } = await openForm(page, "Europe/Berlin");
    await expect(box).toBeVisible();
    await page.clock.pauseAt(new Date("2027-05-10T21:59:00Z"));
    const opened = await box.inputValue();
    await page.clock.runFor(120_000);
    await expect(box).toHaveValue(addDays(opened, 1));
  });
});
