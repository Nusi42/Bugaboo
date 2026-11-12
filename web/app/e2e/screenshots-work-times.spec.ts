import { t } from "@bugaboo/messages";

import {
  addressInProject,
  paths,
  personAddress,
  personWorkTimesAddress,
} from "../src/shared/paths.ts";
import { giveRole, lea } from "./support/access.ts";
import { ada, bob, cleo, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { signIn, signOut } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { type Instance } from "./support/server.ts";
import {
  addAwayTime,
  dayFromToday,
  idOf,
  openAddAwayTime,
  prepareWorkTimes,
  setShareOf,
} from "./support/work-times.ts";

// The screens of work times in the four themes, on a computer and a phone (PLAN-11, PLAN-15, UX
// §4.13): one's own page with away times and shares over 100 %, the window *Add away time*, the same
// page and window for somebody else, the entry on the person's page, *Project members*, and the
// organization's *Holidays*. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWorkTimes } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M8/work-times";

/** Bob in OTHER too, with shares that add up to more than 100 %, and an away time entered by Ada. */
async function bobsShares(server: Instance): Promise<string> {
  const bobId = await idOf(server, bob);
  const admin = await signInThroughApi(server, ada);
  await giveRole(admin, bobId, t("starting-role.team-member"), "OTHER");
  await admin.dispose();
  await setShareOf(server, ada, "WEB", bobId, 60);
  await setShareOf(server, ada, "OTHER", bobId, 70);
  return bobId;
}

test("the own page of work times with shares, and project members", async ({ page, server }) => {
  await bobsShares(server);
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  await addAwayTime(page, {
    from: dayFromToday(14),
    to: dayFromToday(18),
    note: "Course in Berlin",
  });
  await addAwayTime(page, { from: dayFromToday(40), to: dayFromToday(40) });
  await inFourChosenThemes(page, "own-page", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.workTimes);
    await expect(page.getByText("Course in Berlin")).toBeVisible();
  });
  await signOut(page);
  await signIn(page, cleo);
  await inFourChosenThemes(page, "project-members", MILESTONE, paths.appearance, async () => {
    await page.goto(addressInProject(paths.projectMembers, "WEB"));
    await expect(page.getByText(t("project-members.over"))).toBeVisible();
  });
});

test("the window Add away time", async ({ page }) => {
  await signIn(page, bob);
  await inFourChosenThemes(page, "add-away-time", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.workTimes);
    await openAddAwayTime(page);
  });
});

test("the page and the window for somebody else, and the entry on the person's page", async ({
  page,
  server,
}) => {
  const leaId = await idOf(server, lea);
  await signIn(page, ada);
  await inFourChosenThemes(page, "person-entry", MILESTONE, paths.appearance, async () => {
    await page.goto(personAddress(leaId));
    await expect(
      page.getByRole("link", { name: t("work-times.title-other", { name: lea.name }) }),
    ).toBeVisible();
  });
  await page.goto(personWorkTimesAddress(leaId));
  await addAwayTime(page, { from: dayFromToday(7), to: dayFromToday(9) });
  await inFourChosenThemes(page, "other-page", MILESTONE, paths.appearance, async () => {
    await page.goto(personWorkTimesAddress(leaId));
    await expect(
      page.getByText(t("work-times.changing-other", { name: lea.name }), { exact: true }),
    ).toBeVisible();
  });
  await inFourChosenThemes(page, "other-window", MILESTONE, paths.appearance, async () => {
    await page.goto(personWorkTimesAddress(leaId));
    await openAddAwayTime(page);
  });
});

test("the organization's choice of Holidays on Working hours", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(
    page,
    "working-hours-holidays",
    MILESTONE,
    paths.appearance,
    async () => {
      await page.goto(paths.workingHours);
      await expect(
        page.getByLabel(t("setting.work-times.holidays.name"), { exact: true }),
      ).toBeVisible();
    },
  );
});
