import { t } from "@bugaboo/messages";

import { addressInProject, paths, personWorkTimesAddress } from "../src/shared/paths.ts";
import { ada, bob, cleo } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  addAwayTime,
  awayWindow,
  dayFromToday,
  idOf,
  prepareWorkTimes,
} from "./support/work-times.ts";

// The accessibility suite for work times (TEST-A11Y): `paths.workTimes` with an away time,
// `paths.personWorkTimes` for somebody else, the window *Add away time*, reached and left with the
// keyboard only, and `paths.projectMembers` with the shares. Each in the four themes on a computer
// and a phone.

test.use({ preparation: { run: prepareWorkTimes } });

test("a11y the own work times at paths.workTimes with an away time", async ({ page }) => {
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  await addAwayTime(page, { from: dayFromToday(20), to: dayFromToday(22), note: "Course" });
  await expect(page.getByText("Course")).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the window Add away time, with the keyboard only", async ({ page }) => {
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  const add = page.getByRole("button", { name: t("work-times.away-add"), exact: true });
  await add.focus();
  await page.keyboard.press("Enter");
  const window = awayWindow(page);
  await expect(window).toBeVisible();
  // The first box has the focus, and every box and choice is reached with Tab.
  await expect(window.getByLabel(t("work-times.from"), { exact: true })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    window.getByRole("radio", { name: t("work-times.whole-day") }).first(),
  ).toBeFocused();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
  await expect(add).toBeFocused();
});

test("a11y the work times of somebody else at paths.personWorkTimes", async ({ page, server }) => {
  const bobId = await idOf(server, bob);
  await signIn(page, ada);
  await page.goto(personWorkTimesAddress(bobId));
  await expect(
    page.getByText(t("work-times.changing-other", { name: bob.name }), { exact: true }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the shares at paths.projectMembers", async ({ page }) => {
  await signIn(page, cleo);
  await page.goto(addressInProject(paths.projectMembers, "WEB"));
  await expect(
    page.getByRole("textbox", { name: t("project-members.share-of", { name: bob.name }) }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});
