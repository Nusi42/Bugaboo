import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { dayFromToday, goalTitle, phoneItemOf, phoneList, planned } from "./support/timeline.ts";
import { datesAs } from "./support/timeline-api.ts";
import { people, prepareTimelinePeople, warningsButton } from "./support/timeline-people.ts";

// The timeline on a phone (UX-02, UX §4.7, M8-Q12): a list grouped like the rows on a computer, each
// issue with its dates, its person and a short bar over seven weeks; the card and the warnings as
// sheets; dates set with *Set dates…*, never by dragging.

test.use({ preparation: { run: prepareTimelinePeople } });

/** Opens the search of WEB as a timeline on the phone. */
async function openPhoneTimeline(page: Page): Promise<void> {
  await page.goto(`/search?s=${encodeURIComponent("project is WEB")}&show=timeline&group=parent`);
  await expect(phoneList(page)).toBeVisible();
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
  const shared = phoneItemOf(page, people.shared);
  await expect(shared).toContainText(`${bob.name} +1`);
  await expect(
    shared.locator('[data-part="timeline-phone.lane"] [data-part="timeline-bar"]'),
  ).toHaveCount(1);
  await expect(shared.locator('[data-part="timeline-today"]')).toHaveCount(1);
  // *Not scheduled yet* is the last group of the list, drawn like the others.
  await expect(phoneList(page).locator('[data-part="timeline-phone.group"]').last()).toContainText(
    t("timeline.not-scheduled"),
  );
  // *Warnings (N)* opens the list as a sheet.
  await warningsButton(page).click();
  const sheet = page.getByRole("dialog", { name: t("timeline.warnings-title") });
  await expect(sheet.getByRole("region", { name: t("timeline.warnings-title") })).toBeVisible();
  await expect(sheet).toContainText(t("timeline.warnings-closing"));
  // Pressing a warning closes the sheet of the warnings, then opens the card of its issue.
  await sheet.locator('[data-part="timeline-warning.sentence"]').first().click();
  await expect(sheet).toBeHidden();
  await expect(page.getByRole("dialog", { name: t("timeline.phone-sheet") })).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(1);
});

test("ux_02_the_tools_of_the_phone_leave_the_list_the_room_and_group_by_is_in_the_filters", async ({
  page,
}) => {
  await signIn(page, ada);
  await openPhoneTimeline(page);
  // Above the list stand only *Dates from*, *Plan by effort* and *Warnings*: the list starts in the
  // upper half of the screen, so more than three issues show (UX §4.7, §6.5).
  const fromTop = await phoneList(page).evaluate(
    (list) => list.getBoundingClientRect().top / window.innerHeight,
  );
  expect(fromTop).toBeLessThan(1 / 2);
  await expect(page.getByLabel(t("timeline.show-as"), { exact: true })).toHaveCount(0);
  await expect(page.getByText(t("timeline.dates-from"), { exact: true })).toBeVisible();
  // *Show as* and *Group by* are in the sheet of the filters, as for a list (UX §4.2), and *Group by*
  // offers what a timeline groups by: the goal, never the status group of a list.
  await page.getByRole("button", { name: t("search.bar.filters-count", { count: 1 }) }).click();
  const sheet = page.getByRole("dialog", { name: t("search.bar.filters") });
  const groupBy = sheet.getByLabel(t("issues.group-by"), { exact: true });
  await expect(groupBy).toHaveValue("parent");
  await expect(groupBy.locator('option[value="status group"]')).toHaveCount(0);
  await expect(sheet.getByLabel(t("timeline.show-as"), { exact: true })).toHaveValue("timeline");
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
  await phoneItemOf(page, planned.keys).click();
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
