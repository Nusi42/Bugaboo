import { t } from "@bugaboo/messages";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { lea } from "./support/access.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn, signOut } from "./support/flows.ts";
import {
  addAwayTime,
  awayWindow,
  comingAway,
  dayFromToday,
  idOf,
  notificationKindsOf,
  openAddAwayTime,
  openWorkTimesOf,
  ownWorkTimes,
  prepareWorkTimes,
} from "./support/work-times.ts";

// *My settings › Work times* and the same page for somebody else (PLAN-11, ADR-0030, UX §4.13):
// away times with no reason, the person told when somebody else enters one, and the next three
// public holidays of the choice of *Holidays*.

test.use({ preparation: { run: prepareWorkTimes } });

test("plan_11_a_person_enters_an_away_time_and_others_see_only_away", async ({ page, server }) => {
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  await addAwayTime(page, { from: dayFromToday(30), to: dayFromToday(34), note: "Dentist" });
  const mine = comingAway(page);
  await expect(mine.getByRole("listitem")).toHaveCount(1);
  // Bob sees his own note.
  await expect(mine.getByText("Dentist")).toBeVisible();
  // The administrator sees the days and nothing else: no note, no reason.
  await signOut(page);
  await signIn(page, ada);
  await openWorkTimesOf(page, bob, await idOf(server, bob));
  const theirs = comingAway(page);
  await expect(theirs.getByRole("listitem")).toHaveCount(1);
  await expect(page.getByText("Dentist")).toHaveCount(0);
});

test("plan_11_the_window_says_once_that_no_reason_is_stored", async ({ page }) => {
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  const window = await openAddAwayTime(page);
  await expect(window.getByText(t("work-times.no-reason"), { exact: true })).toHaveCount(1);
  // The page itself does not say it a second time, and nothing asks for a reason.
  await expect(page.getByText(t("work-times.no-reason"), { exact: true })).toHaveCount(1);
  await expect(window.getByRole("textbox")).toHaveCount(3);
  await expect(window.getByRole("radio", { name: t("work-times.afternoon-only") })).toBeVisible();
  await expect(window.getByRole("radio", { name: t("work-times.morning-only") })).toBeVisible();
  await expect(window.getByLabel(t("work-times.note"), { exact: true })).toBeVisible();
});

test("plan_11_someone_with_enter_work_times_for_others_changes_lea_and_lea_is_told", async ({
  page,
  server,
}) => {
  const leaId = await idOf(server, lea);
  await signIn(page, ada);
  await page.goto(`${paths.people}/${leaId}`);
  await page
    .getByRole("link", { name: t("work-times.title-other", { name: lea.name }), exact: true })
    .click();
  await expect(
    page.getByText(t("work-times.changing-other", { name: lea.name }), { exact: true }),
  ).toBeVisible();
  const window = await openAddAwayTime(page);
  await expect(window.getByText(t("work-times.told", { name: lea.name }))).toBeVisible();
  // There is no note for somebody else.
  await expect(window.getByLabel(t("work-times.note"), { exact: true })).toHaveCount(0);
  await window.getByRole("button", { name: t("common.cancel"), exact: true }).click();
  await addAwayTime(page, { from: dayFromToday(10), to: dayFromToday(10) });
  await expect(comingAway(page).getByRole("listitem")).toHaveCount(1);
  await expect.poll(() => notificationKindsOf(server, lea)).toContain("work-times-changed");
});

test("plan_11_the_holidays_picker_shows_the_next_three_holidays", async ({ page, server }) => {
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  const picker = page.getByLabel(t("setting.work-times.holidays.name"), { exact: true });
  await picker.selectOption({ label: t("holidays.region.none") });
  await expect(page.getByText(t("work-times.holidays-none"), { exact: true })).toBeVisible();
  await picker.selectOption({ label: t("holidays.region.de-by") });
  // The three that come next for Bavaria, as the server works them out.
  await expect.poll(async () => (await ownWorkTimes(server, bob)).holidays).toBe("de-by");
  const read = await ownWorkTimes(server, bob);
  expect(read.next_holidays).toHaveLength(3);
  await Promise.all(
    read.next_holidays.map((day) =>
      expect(page.getByText(new RegExp(t(`holiday.${day.holiday}`), "u"))).toBeVisible(),
    ),
  );
});

test("plan_11_away_times_of_others_cannot_be_reached_without_the_permission", async ({
  page,
  server,
}) => {
  const bobId = await idOf(server, bob);
  // Lea, a Guest, holds neither *Enter work times for others* nor *Decide who has which role*.
  await signIn(page, lea);
  await page.goto(`${paths.people}/${bobId}/work-times`);
  await expect(page.getByRole("heading", { level: 1 })).not.toHaveText(
    t("work-times.title-other", { name: bob.name }),
  );
  await expect(awayWindow(page)).toHaveCount(0);
  await page.goto(addressInProject(paths.projectMembers, "WEB"));
  await expect(page.getByRole("heading", { name: t("nav.project-members") })).toHaveCount(0);
});
