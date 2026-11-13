import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { topOf } from "./support/geometry.ts";
import { dayFromToday, goalTitle, planned } from "./support/timeline.ts";
import { datesAs } from "./support/timeline-api.ts";
import { people, prepareTimelinePeople, warningsButton } from "./support/timeline-people.ts";

// The timeline on a phone (UX-02, UX §4.7, M8-Q12): a list grouped like the rows on a computer, each
// issue with its dates, its person and a short bar over seven weeks; the card and the warnings as
// sheets; dates set with *Set dates…*, never by dragging.

test.use({ preparation: { run: prepareTimelinePeople } });

/** The list of the timeline on a phone. */
function phoneList(page: Page) {
  return page.getByRole("list", { name: t("timeline.phone-list"), exact: true });
}

/** Opens the search of WEB as a timeline on the phone. */
async function openPhoneTimeline(page: Page): Promise<void> {
  await page.goto(`/search?s=${encodeURIComponent("project is WEB")}&show=timeline&group=parent`);
  await expect(phoneList(page)).toBeVisible();
}

/** The item of the list of the issue with this readable ID. */
function itemOf(page: Page, key: string) {
  return phoneList(page)
    .locator('[data-part="timeline-phone.item"]')
    .filter({ has: page.getByText(key, { exact: true }) });
}

test("ux_02_the_phone_timeline_is_a_grouped_list_with_dates_person_and_a_seven_week_bar", async ({
  page,
}) => {
  await signIn(page, ada);
  await openPhoneTimeline(page);
  // Grouped like the chart: the goal heads its issues.
  const goal = phoneList(page).locator('[data-part="timeline-phone.group"]', {
    hasText: goalTitle,
  });
  await expect(goal.getByRole("button")).toHaveAttribute("aria-expanded", "true");
  // Each issue: its title, its dates, its person, and the short bar with today marked.
  const shared = itemOf(page, people.shared);
  await expect(shared).toContainText(`${bob.name} +1`);
  await expect(
    shared.locator('[data-part="timeline-phone.lane"] [data-part="timeline-bar"]'),
  ).toHaveCount(1);
  await expect(shared.locator('[data-part="timeline-today"]')).toHaveCount(1);
  // *Not scheduled yet* is the last group, under the list.
  const tray = page.getByRole("region", { name: t("timeline.not-scheduled") });
  expect(await topOf(tray)).toBeGreaterThan(await topOf(phoneList(page)));
  // *Warnings (N)* opens the list as a sheet.
  await warningsButton(page).click();
  const sheet = page.getByRole("dialog", { name: t("timeline.warnings-title") });
  await expect(sheet.getByRole("region", { name: t("timeline.warnings-title") })).toBeVisible();
  await expect(sheet).toContainText(t("timeline.warnings-closing"));
});

test("ux_02_on_a_phone_set_dates_changes_the_dates_and_nothing_is_dragged", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openPhoneTimeline(page);
  // Nothing on the phone can be dragged: no bar to press and no handle in the tray.
  await expect(page.locator('[data-part="timeline-bar.press"]')).toHaveCount(0);
  await expect(page.locator('[data-part="draggable.handle"]')).toHaveCount(0);
  // Tapping an issue opens its card as a sheet, with *Set dates…*.
  await itemOf(page, planned.keys).click();
  const sheet = page.getByRole("dialog", { name: t("timeline.phone-sheet") });
  await sheet.getByRole("button", { name: t("timeline.set-dates"), exact: true }).click();
  const window = page.getByRole("dialog", {
    name: t("timeline.set-dates-title", { key: planned.keys }),
  });
  await window.getByLabel(t("issue-field.start-date")).fill(dayFromToday(10));
  await window.getByLabel(t("issue-field.due-date")).fill(dayFromToday(11));
  await window.getByRole("button", { name: t("timeline.save-dates") }).click();
  await expect
    .poll(() => datesAs(server, ada, planned.keys))
    .toEqual({ start: dayFromToday(10), due: dayFromToday(11) });
});
