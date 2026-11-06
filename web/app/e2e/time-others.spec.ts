import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ana } from "./support/access.ts";
import { signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { ben, hana } from "./support/hr.ts";
import { setOrganizationSwitch } from "./support/organization.ts";
import { sheetAt, tabThrough } from "./support/timesheet.ts";
import { anaWorked, PAST, prepareTimesheet } from "./support/timesheet-data.ts";

// Working hours of others (TIME-12, PRIV-09, ADR-0035, decision 8): the page is for a holder of
// See working hours of others; everybody else is answered as if it did not exist. While Totals
// only is on, a week is its total and nothing else.

/** Enough presses of Tab to go over the menu and the page up to its last button. */
const TABS = 40;

test.use({ preparation: { run: prepareTimesheet } });

test("time_12 ben cannot open anas working hours", async ({ page, server }) => {
  const anaId = await anaWorked(server);
  await signIn(page, ben);
  // The page answers like a page that does not exist, and the menu has no entry and no picker.
  await page.goto(`/time/people/${anaId}?week=${PAST}`);
  await expect(page.getByRole("heading", { name: t("page.not-found.title") })).toBeVisible();
  await page.goto("/time/people");
  await expect(page.getByRole("heading", { name: t("page.not-found.title") })).toBeVisible();
  await expect(page.getByRole("link", { name: t("time.others.title") })).toHaveCount(0);
  const own = await signInThroughApi(server, ben);
  await expect(
    own.client.GET("/api/v1/people/{person}/work-days", {
      params: { path: { person: anaId }, query: { from: PAST, to: PAST } },
    }),
  ).rejects.toMatchObject({ status: 404 });
  await expect(own.client.GET("/api/v1/work-days/people")).rejects.toMatchObject({ status: 403 });
  await own.dispose();
});

test("time_12 hr sees anas week and with totals only only the week total", async ({
  page,
  server,
}) => {
  const anaId = await anaWorked(server);
  await signIn(page, hana);
  await page.goto("/time/people");
  await page
    .getByRole("button", { name: t("time.others.pick-take", { person: ana.name }) })
    .click();
  await expect(page).toHaveURL(new RegExp(anaId, "u"));
  await page.goto(`/time/people/${anaId}${sheetAt(PAST).slice("/time".length)}`);
  await expect(page.getByRole("heading", { name: t("time.others.title") })).toBeVisible();
  // Looked at, not changed, until Change; Ana's list of time on issues is not here.
  await expect(page.getByRole("textbox").first()).toHaveAttribute("readonly", "");
  await expect(page.getByText(t("time.sheet.time-on-issues"), { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: t("time.others.change"), exact: true }).click();
  await expect(page.getByRole("textbox").first()).not.toHaveAttribute("readonly", "");
  // The total of the week is the one figure of the week.
  await expect(page.getByText("8:50", { exact: true }).first()).toBeVisible();
  await setOrganizationSwitch(server, settingKeys.workingHoursTotalsOnly, true);
  await page.reload();
  await expect(page.getByText(/8 hours 50 minutes worked/u)).toBeVisible();
  await expect(page.getByRole("textbox")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: t("time.others.change"), exact: true }),
  ).toHaveCount(0);
  await expect(page.getByText(t("time.sheet.start"), { exact: true })).toHaveCount(0);
});

test("time_12 with totals only the page is reached by keyboard and has no box to type in", async ({
  page,
  server,
}) => {
  const anaId = await anaWorked(server);
  await setOrganizationSwitch(server, settingKeys.workingHoursTotalsOnly, true);
  await signIn(page, hana);
  await page.goto(`/time/people/${anaId}${sheetAt(PAST).slice("/time".length)}`);
  await expect(page.getByText(/8 hours 50 minutes worked/u)).toBeVisible();
  // Tab reaches *Choose another person*, and nothing on the way is a box to type in.
  const reached = await tabThrough(page, TABS);
  expect(reached.texts.join("|")).toContain(t("time.others.other-person"));
  expect(reached.boxes).toBe(0);
  await expect(page.getByRole("textbox")).toHaveCount(0);
  await expect(page.getByRole("button", { name: t("time.others.other-person") })).toBeVisible();
  // Only the sentence of the week total shows: no day, no start, end or breaks.
  await expect(page.getByRole("table")).toHaveCount(0);
  await expect(page.getByText(t("time.sheet.start"), { exact: true })).toHaveCount(0);
  await expect(page.getByText(t("time.sheet.breaks"), { exact: true })).toHaveCount(0);
});
