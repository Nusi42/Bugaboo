import { ada, signInThroughApi } from "./support/accounts.ts";
import {
  calendarOf,
  dayOf,
  dragToDay,
  itemOf,
  pressOf,
  weekDays,
} from "./support/calendar-places.ts";
import { openCalendar, prepareCalendar, scene } from "./support/calendar.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { boxOf } from "./support/geometry.ts";
import { takeUp } from "./support/hand-drag.ts";
import { expectStill, landmarkOf, movementWhile, partLandmark } from "./support/movement.ts";
import { changeIssue } from "./support/timeline.ts";
import { datesAs } from "./support/timeline-api.ts";

// A drop with the pointer lands on the day under it, on any part of the day (PLAN-09, UX §4.16): its empty part, over
// one of its issues, over the menu of an issue, over *+N more*. A day of three lines that gets a fourth moves nothing.

test.use({ preparation: { run: prepareCalendar } });

/** The parts of the crowded day (six issues, so *+3 more* stands in it) where the pointer lets go. */
const parts = [
  "its corner",
  "its middle",
  "its foot",
  "an issue",
  "the menu of an issue",
  "+N more",
] as const;

/** Where the pointer is over the day for one of its parts. */
async function placeOf(
  page: Parameters<typeof dayOf>[0],
  day: string,
  part: (typeof parts)[number],
) {
  const cell = await boxOf(dayOf(page, day));
  const item = await boxOf(itemOf(page, scene.crowdFirst));
  const more = await boxOf(dayOf(page, day).getByRole("button", { name: /^\+\d+ more on /u }));
  const places = {
    "its corner": { x: cell.x + cell.width - 6, y: cell.y + 4 },
    "its middle": { x: cell.x + cell.width / 2, y: cell.y + cell.height / 2 },
    "its foot": { x: cell.x + cell.width / 2, y: cell.y + cell.height - 4 },
    "an issue": { x: item.x + item.width / 2, y: item.y + item.height / 2 },
    "the menu of an issue": { x: item.x + item.width - 8, y: item.y + item.height / 2 },
    "+N more": { x: more.x + more.width / 2, y: more.y + more.height / 2 },
  };
  return places[part];
}

for (const part of parts) {
  test(`plan_09_a_drop_over_${part.replaceAll(/\W+/gu, "_")}_of_a_day_lands_on_that_day`, async ({
    page,
    server,
  }) => {
    const { crowded, today } = weekDays();
    const admin = await signInThroughApi(server, ada);
    await changeIssue(admin, scene.dragged, { due_date: today });
    await admin.dispose();
    await signIn(page, ada);
    await openCalendar(page);
    await dayOf(page, crowded).scrollIntoViewIfNeeded();
    const at = await placeOf(page, crowded, part);
    await takeUp(page, pressOf(page, scene.dragged));
    await page.mouse.move(at.x, at.y, { steps: 12 });
    await page.mouse.move(at.x + 1, at.y);
    await page.mouse.move(at.x, at.y);
    await page.mouse.up();
    await expect.poll(async () => (await datesAs(server, ada, scene.dragged)).due).toBe(crowded);
  });
}

test("plan_09_a_pointer_drop_on_a_day_of_three_lines_moves_nothing_but_the_issue", async ({
  page,
  server,
}) => {
  const { crowded, other, next } = weekDays();
  const admin = await signInThroughApi(server, ada);
  // The crowded day keeps WEB-1 to WEB-3; the issue that joins them (it sorts after them) stands alone on another.
  await Promise.all([
    ...scene.crowd.slice(3).map((key) => changeIssue(admin, key, { due_date: next })),
    changeIssue(admin, scene.dragged, { due_date: other }),
  ]);
  await admin.dispose();
  await signIn(page, ada);
  await openCalendar(page);
  // The day is scrolled to before anything is measured: the scroll is the test's, not the page's.
  await dayOf(page, crowded).scrollIntoViewIfNeeded();
  const grid = await landmarkOf(calendarOf(page), "grid");
  const tools = partLandmark("tools", "calendar-tools");
  const kept = await landmarkOf(
    itemOf(page, scene.crowdFirst),
    "an issue of the day that gets a fourth",
  );
  const more = dayOf(page, crowded).getByRole("button", { name: /^\+1 more on /u });
  await expect(more).toHaveCount(0);
  const dropping = await movementWhile(page, [grid, tools, kept], async () => {
    await dragToDay(page, scene.dragged, dayOf(page, crowded));
    await expect(more).toBeVisible();
  });
  expectStill(dropping, {});
});

test("plan_09_dragging_to_the_foot_of_the_box_scrolls_it_to_a_day_out_of_view", async ({
  page,
  server,
}) => {
  // The first and the last day of the month stand in the first and the last row, far apart in a box that shows two.
  const first = `${weekDays().today.slice(0, 8)}01`;
  const month = new Date(`${first}T00:00:00Z`);
  month.setUTCMonth(month.getUTCMonth() + 1, 0);
  const last = month.toISOString().slice(0, 10);
  const admin = await signInThroughApi(server, ada);
  await changeIssue(admin, scene.dragged, { due_date: first });
  await admin.dispose();
  await signIn(page, ada);
  await openCalendar(page);
  const box = page.locator('[data-part="calendar-scroll"]');
  const frame = await boxOf(box);
  const top = (await boxOf(dayOf(page, last))).y;
  expect(top, "the last day starts below the box").toBeGreaterThan(frame.y + frame.height);
  // The pointer takes the issue and rests near the foot of the box, as a hand does; no scrollIntoView first. The browser
  // scrolls a box a drag rests at the edge of (checked here: no code of ours does it), so the day comes into view.
  await takeUp(page, pressOf(page, scene.dragged));
  const foot = { x: frame.x + frame.width / 2, y: frame.y + frame.height - 6 };
  await page.mouse.move(foot.x, foot.y, { steps: 8 });
  await expect
    .poll(async () => {
      await page.mouse.move(foot.x + 1, foot.y);
      await page.mouse.move(foot.x, foot.y);
      return (await boxOf(dayOf(page, last))).y;
    })
    .toBeLessThan(frame.y + frame.height - 20);
  const at = await boxOf(dayOf(page, last));
  await page.mouse.move(at.x + at.width / 2, at.y + 6, { steps: 8 });
  await page.mouse.move(at.x + at.width / 2 + 1, at.y + 6);
  await page.mouse.up();
  await expect.poll(async () => (await datesAs(server, ada, scene.dragged)).due).toBe(last);
});
