import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { hana } from "./support/hr.ts";
import { historyOfThursday, openDay, openWeek } from "./support/timesheet.ts";
import { anaWorked, bobsWeek, PAST, prepareTimesheet } from "./support/timesheet-data.ts";
import { switchTotalsOnly } from "./support/timesheet-pictures.ts";

// The accessibility suite for the timesheet (TEST-A11Y): `paths.time`, `paths.timeOthers` and
// `paths.timeOfPerson` in the states a person meets: a recorded week of one's own, a day with a
// warning, a cell with a mistake, the reason window, the history window, the week of another
// person and Totals only. Each in the four themes on a computer and a phone.

test.use({ preparation: { run: prepareTimesheet } });

test("a11y the timesheet at paths.time with a recorded week and a warning", async ({
  page,
  server,
}) => {
  await bobsWeek(server);
  await signIn(page, bob);
  await openDay(page, PAST, 3);
  await expect(page.getByRole("heading", { level: 1, name: t("time.sheet.title") })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y a cell with a mistake", async ({ page, server }) => {
  await bobsWeek(server);
  await signIn(page, bob);
  await openDay(page, PAST, 3);
  const start = page.getByRole("textbox", { name: /^Start, Thursday/u });
  await start.fill("soon");
  await start.press("Tab");
  await expect(start).toHaveAttribute("aria-invalid", "true");
  await expectGoodInEveryLook(page);
});

test("a11y the reason window", async ({ page, server }) => {
  await bobsWeek(server);
  await signIn(page, bob);
  await openDay(page, PAST, 3);
  const end = page.getByRole("textbox", { name: /^End, Thursday/u });
  await end.fill("17:45");
  await end.press("Tab");
  await expect(page.getByRole("dialog")).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the history window of a changed day", async ({ page, server }) => {
  await bobsWeek(server);
  await signIn(page, bob);
  await openDay(page, PAST, 3);
  await historyOfThursday(page, PAST).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the picker at paths.timeOthers", async ({ page }) => {
  await signIn(page, hana);
  await page.goto(paths.timeOthers);
  await expect(page.getByRole("heading", { level: 1, name: t("time.others.title") })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the week of one person at paths.timeOfPerson", async ({ page, server }) => {
  const person = await anaWorked(server);
  await signIn(page, hana);
  await page.goto(`/time/people/${person}?week=${PAST}`);
  await expect(page.getByRole("heading", { level: 1, name: t("time.others.title") })).toBeVisible();
  await expect(page.getByRole("textbox").first()).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the week of one person with totals only", async ({ page, server }) => {
  const person = await anaWorked(server);
  await switchTotalsOnly(server);
  await signIn(page, hana);
  await page.goto(`/time/people/${person}?week=${PAST}`);
  await expect(page.getByText(/worked/u).first()).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the empty own week", async ({ page }) => {
  await signIn(page, bob);
  await openWeek(page, "2026-08-03");
  await expect(page.getByText(t("time.sheet.empty"))).toBeVisible();
  await expectGoodInEveryLook(page);
});
