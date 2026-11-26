import { dayInSentence } from "../src/shared/clock.ts";
import { ada, signInThroughApi } from "./support/accounts.ts";
import {
  calendarOf,
  dayOf,
  dragToDay,
  itemOf,
  keysTo,
  movedTo,
  pressOf,
  weekDays,
} from "./support/calendar-places.ts";
import { openCalendar, openProjectCalendar, prepareCalendar, scene } from "./support/calendar.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { expectStill, landmarkOf, movementWhile, partLandmark } from "./support/movement.ts";
import { inOrder } from "./support/sequence.ts";
import { changeIssue } from "./support/timeline.ts";

// The calendar stands still while a person acts, and the keys do what the pointer does (PLAN-09, UX §4.16, §6.4):
// *Enter* opens an issue and leaves the page usable, a drop on a full day and the carrying over one move nothing but
// the issue, a change that somebody made first asks, and *Undo* follows a put-down.

test.use({ preparation: { run: prepareCalendar } });

test("plan_09_enter_on_an_issue_opens_it_and_leaves_the_page_usable", async ({ page }) => {
  await signIn(page, ada);
  await openProjectCalendar(page);
  await pressOf(page, scene.child).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: /Child of the goal/u })).toBeVisible();
  // No drag session stands over the page: nothing of the page is inert, and the next Tab stays inside the issue page.
  await expect(page.locator("main[inert], main [inert]")).toHaveCount(0);
  await page.keyboard.press("Tab");
  await expect(page.locator(":focus")).toBeVisible();
  await expect(page.locator("main").locator(":focus")).toHaveCount(1);
});

test("plan_09_a_drop_on_a_full_day_and_carrying_over_it_move_nothing_but_the_issue", async ({
  page,
}) => {
  const { today, crowded } = weekDays();
  await signIn(page, ada);
  await openCalendar(page);
  const grid = await landmarkOf(calendarOf(page), "grid");
  const tools = partLandmark("tools", "calendar-tools");
  const kept = await landmarkOf(itemOf(page, scene.crowdFirst), "an issue of the full day");
  // Carrying: the full day takes the place of the issue as one of its lines, and its height stays.
  await pressOf(page, scene.typed).focus();
  const carrying = await movementWhile(page, [grid, tools, kept], async () => {
    await page.keyboard.press("Space");
    await inOrder(keysTo(today, crowded).map((key) => () => page.keyboard.press(key)));
    await expect(dayOf(page, crowded).locator('[data-state~="ghost"]')).toBeVisible();
  });
  expectStill(carrying, {});
  await page.keyboard.press("Escape");
  // Dropping on the full day with the pointer.
  const dropping = await movementWhile(page, [grid, tools], async () => {
    await dragToDay(page, scene.dragged, dayOf(page, crowded));
    await expect(page.getByText(movedTo(scene.dragged, dayInSentence(crowded)))).toBeVisible();
  });
  expectStill(dropping, {});
});

test("plan_09_a_day_of_three_lines_that_gets_a_fourth_moves_nothing_but_the_issue", async ({
  page,
  server,
}) => {
  const { crowded, other, next } = weekDays();
  const admin = await signInThroughApi(server, ada);
  // The crowded day keeps WEB-1 to WEB-3, the issue that will join them (it sorts after them) stands alone on another.
  await inOrder([
    ...scene.crowd.slice(3).map((key) => () => changeIssue(admin, key, { due_date: next })),
    () => changeIssue(admin, scene.dragged, { due_date: other }),
  ]);
  await admin.dispose();
  await signIn(page, ada);
  await openCalendar(page);
  const grid = await landmarkOf(calendarOf(page), "grid");
  const tools = partLandmark("tools", "calendar-tools");
  const kept = await landmarkOf(
    itemOf(page, scene.crowdFirst),
    "an issue of the day that gets a fourth",
  );
  const moreOfOther = dayOf(page, crowded).getByRole("button", { name: /\+1/u });
  await expect(moreOfOther).toHaveCount(0);
  await pressOf(page, scene.dragged).focus();
  // The fourth line makes *+1 more* appear for the first time: the day keeps its height.
  const dropping = await movementWhile(page, [grid, tools, kept], async () => {
    await page.keyboard.press("Space");
    await inOrder(keysTo(other, crowded).map((key) => () => page.keyboard.press(key)));
    await page.keyboard.press("Space");
    await expect(moreOfOther).toBeVisible();
  });
  expectStill(dropping, {});
});
