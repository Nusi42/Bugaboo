import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { expectStill, landmarkOf, movementWhile } from "./support/movement.ts";
import { phoneItemOf, phoneList, planned } from "./support/timeline.ts";
import { prepareDrivers, release } from "./support/timeline-drivers.ts";

// Deadline drivers and releases on a phone (PLAN-05, ISS-08, M8-Q12): grouped by release, the head of
// the release shows its diamond and its *Release date*, and a driver says *Sets the end* in words.

test.use({ preparation: { run: prepareDrivers } });

test("plan_05_the_phone_list_marks_drivers_and_release_diamonds", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(
    `/search?s=${encodeURIComponent("project is WEB")}&show=timeline&group=${encodeURIComponent("planned for release")}&drivers=1`,
  );
  await expect(phoneList(page)).toBeVisible();
  const head = phoneList(page).locator('[data-part="timeline-phone.group"]', {
    hasText: release.name,
  });
  await expect(head.locator('[data-part="timeline-phone.release"]')).toContainText(
    t("timeline.release-date", { day: "" }).trim(),
  );
  await expect(head.locator('[data-part="timeline-release.diamond"]')).toHaveCount(1);
  // The issue planned for the release is a driver of its end, said with a word.
  await expect(phoneItemOf(page, planned.waits)).toContainText(t("timeline.sets-end-short"));
  // The group of no release comes last, before *Not scheduled yet*.
  const heads = phoneList(page).locator('[data-part="timeline-phone.group"]');
  await expect(heads.first()).toContainText(release.name);
  await expect(heads.nth((await heads.count()) - 2)).toContainText(t("timeline.no-release"));
  await expect(heads.last()).toContainText(t("timeline.not-scheduled"));
});

test("plan_05_switching_deadline_drivers_on_moves_nothing_in_the_phone_list", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(`/search?s=${encodeURIComponent("project is WEB")}&show=timeline&group=parent`);
  await expect(phoneList(page)).toBeVisible();
  const marks = [
    await landmarkOf(phoneItemOf(page, planned.first), "first item"),
    await landmarkOf(phoneItemOf(page, planned.dragged), "an item below"),
  ];
  const switching = await movementWhile(page, marks, async () => {
    await page.getByText(t("timeline.deadline-drivers"), { exact: true }).click();
    await expect(phoneItemOf(page, planned.first)).toContainText(t("timeline.sets-end-short"));
  });
  expectStill(switching);
});
