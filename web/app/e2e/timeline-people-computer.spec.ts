import { t } from "@bugaboo/messages";

import { ana } from "./support/access.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { widthOf } from "./support/geometry.ts";
import { barOf, chart, openTimeline, pressOf } from "./support/timeline.ts";
import { formatCalendarDay, keepingHyphens } from "../src/shared/format.ts";
import {
  awayDay,
  openWarnings,
  people,
  prepareTimelinePeople,
  setBobsShare,
  viewersSeeAssigned,
  warningAbout,
} from "./support/timeline-people.ts";

// The people on the timeline on a computer (PLAN-10 … PLAN-12, UX §4.7, ADR-0030 and its amendment
// M8): the person on a bar as *Who sees people* allows, *Group by person* with its copies, the
// hatched days without work, and *Plan by effort*.

test.use({ preparation: { run: prepareTimelinePeople } });

/** The field the rows are grouped by, in the address: *Assigned to*. */
const byPerson = `&group=${encodeURIComponent("assigned to")}`;

test("plan_10_a_role_only_reader_sees_someone_and_no_group_by_person", async ({ page, server }) => {
  await viewersSeeAssigned(server, "role-only");
  // Ana is a Viewer of WEB: she sees who works on a bar only by their role.
  await signIn(page, ana);
  await openTimeline(page);
  const someone = t("person.role", { role: t("starting-role.team-member") });
  await expect(barOf(page, people.shared)).toContainText(`${someone} +1`);
  await expect(barOf(page, people.shared)).not.toContainText(bob.name);
  // *Group by* offers no person: nobody could be told apart.
  const groupBy = page.getByLabel(t("issues.group-by"));
  await expect(groupBy.locator("option", { hasText: t("issue-field.assigned-to") })).toHaveCount(0);
  // With *Hidden* no person is on the bar at all.
  await viewersSeeAssigned(server, "hidden");
  await page.reload();
  await expect(barOf(page, people.shared)).not.toContainText(someone);
});

test("plan_10_group_by_person_draws_the_other_assigned_people_as_an_outline_with_its_words", async ({
  page,
}) => {
  await signIn(page, ada);
  await openTimeline(page, byPerson);
  // The bar stands in Bob's row and in Ada's; Ada's copy is an outline planned with Bob's work times.
  const bars = barOf(page, people.shared);
  await expect(bars).toHaveCount(2);
  const copy = bars.and(page.locator('[data-state~="copy"]'));
  await expect(copy).toHaveCount(1);
  const words = t("timeline.planned-with", { name: bob.name });
  await expect(copy.locator('[data-part="timeline-bar.press"]')).toHaveAccessibleName(
    new RegExp(words, "u"),
  );
  // The card of the copy says it too.
  await copy.locator('[data-part="timeline-bar.press"]').click();
  const card = page.getByRole("region", { name: t("timeline.card", { key: people.shared }) });
  await expect(card).toContainText(words);
});

test("plan_11_away_days_are_hatched_and_labelled_away_and_holidays_carry_their_name", async ({
  page,
}) => {
  await signIn(page, ada);
  await openTimeline(page);
  // Bob's away day: hatched, and called only *Away*.
  const shared = barOf(page, people.shared).first();
  await expect(shared.locator('[data-part="timeline-bar.hatch"][data-variant="away"]')).toHaveCount(
    1,
  );
  await expect(pressOf(page, people.shared).first()).toHaveAccessibleName(
    new RegExp(t("timeline.away"), "u"),
  );
  // Over Christmas, at the zoom of months: the holidays hatched, each with its name.
  await openTimeline(page, "&zoom=month");
  const holiday = barOf(page, people.holiday);
  await expect(
    holiday.locator('[data-part="timeline-bar.hatch"][data-variant="holiday"]'),
  ).not.toHaveCount(0);
  await expect(pressOf(page, people.holiday)).toHaveAccessibleName(
    new RegExp(t("timeline.holiday", { name: t("holiday.christmas-day") }), "u"),
  );
});

test("plan_12_plan_by_effort_doubles_the_bars_when_the_share_is_halved", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openTimeline(page);
  // Without *Plan by effort* an issue with only a start date is one day long.
  // The whole bar, edges included: as wide as its days.
  const bar = barOf(page, people.effort);
  const oneDay = await widthOf(bar);
  // The switch is pressed where the eye sees it: its words.
  await page.getByText(t("timeline.plan-by-effort"), { exact: true }).click();
  await expect(page.getByRole("switch", { name: t("timeline.plan-by-effort") })).toBeChecked();
  // 16 hours at 8 hours a day: two work days.
  await expect.poll(() => widthOf(bar)).toBe(oneDay * 2);
  const full = await widthOf(bar);
  // At half of his hours in WEB, Bob needs twice the days.
  await setBobsShare(server, 50);
  await page.reload();
  await expect.poll(() => widthOf(bar)).toBe(full * 2);
  // The card says the effort in work days.
  await pressOf(page, people.effort).click();
  const card = page.getByRole("region", { name: t("timeline.card", { key: people.effort }) });
  await expect(card).toContainText(t("timeline.worked-out-due"));
});

test("plan_13_a_reader_who_sees_people_by_role_or_not_at_all_reads_no_name_in_the_warnings", async ({
  page,
  server,
}) => {
  // Ana is a Viewer of WEB: with Role only, a warning about Bob says "the assigned person", never
  // his name (the server picks the sentence without a person).
  await viewersSeeAssigned(server, "role-only");
  await signIn(page, ana);
  await openTimeline(page);
  const list = await openWarnings(page);
  const someone = t("person.role", { role: t("starting-role.team-member") });
  await expect(list).not.toContainText(bob.name);
  await expect(list).not.toContainText(someone);
  // With Hidden: the assigned person is away, without who; no bar says Away or names a holiday.
  await viewersSeeAssigned(server, "hidden");
  await page.reload();
  const hidden = await openWarnings(page);
  await expect(hidden).not.toContainText(bob.name);
  await expect(hidden).not.toContainText(someone);
  // No away day: the sentence says only that the assigned person is away, and the only way out is
  // Keep and accept, since Move it after and Make it longer would tell the days.
  await expect(hidden).toContainText(
    t("plan-warning.away-hidden", { issue: keepingHyphens(people.shared) }),
  );
  await expect(hidden).not.toContainText(formatCalendarDay(awayDay));
  const away = warningAbout(hidden, people.shared);
  await expect(away.locator('[data-part="timeline-warning.ways"] button')).toHaveText([
    t("plan-way-out.keep-and-accept"),
  ]);
  await expect(pressOf(page, people.shared)).not.toHaveAccessibleName(
    new RegExp(t("timeline.away"), "u"),
  );
  await expect(pressOf(page, people.shared)).not.toHaveAccessibleName(new RegExp(bob.name, "u"));
});

test("plan_11_the_row_of_a_person_hatches_their_away_days_across_the_chart", async ({ page }) => {
  await signIn(page, ada);
  await openTimeline(page, byPerson);
  // Bob's row: his away day hatched in the row of the group, and said in its words.
  const head = chart(page)
    .locator('[data-part="timeline-row"][data-variant~="group"]')
    .filter({ hasText: bob.name });
  await expect(
    head.locator('[data-part="timeline-bar.hatch"][data-variant="away"]'),
  ).not.toHaveCount(0);
  await expect(head.getByRole("button")).toContainText(t("timeline.away"));
  // Every row of Bob's group hatches the day too, behind its bar.
  const row = chart(page)
    .locator('[data-part="timeline-row"]')
    .filter({ has: page.getByText(people.effort, { exact: true }) })
    .first();
  await expect(
    row.locator('[data-part="timeline-row.lane"] > [data-part="timeline-bar.hatch"]'),
  ).not.toHaveCount(0);
});
