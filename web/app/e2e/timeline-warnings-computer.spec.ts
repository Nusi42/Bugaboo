import { t } from "@bugaboo/messages";

import { keepingHyphens } from "../src/shared/format.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { dayFromToday, openTimeline, planned } from "./support/timeline.ts";
import { datesAs, firstAssigned } from "./support/timeline-api.ts";
import {
  openWarnings,
  people,
  prepareTimelinePeople,
  warningAbout,
  warningsButton,
} from "./support/timeline-people.ts";

// The list of plan warnings (PLAN-13, UX §4.7, ADR-0030 and its amendment M8, ADR-0043): one sentence
// of the server for each warning, its ways out, a preview before anything moves, the usual person
// picker with nobody suggested, and *Keep and accept*, which fades a warning with who accepted it.
// The tests of the file share one server and go in order: each takes a warning of its own.

test.use({ preparation: { run: prepareTimelinePeople } });

/** The sentence of the *Delay* of WEB-2: WEB-1, which it waits for, ends a day after WEB-2 starts. */
const delaySentence = t("plan-warning.delay", {
  first: planned.first,
  days: 1,
  issue: planned.waits,
});

/** The way out of a warning, by its words. */
function way(
  name: "move-it-after" | "make-it-longer" | "give-to-someone-else" | "keep-and-accept",
) {
  return { name: t(`plan-way-out.${name}`), exact: true } as const;
}

test("plan_13_each_warning_offers_its_ways_out_and_keep_and_accept_fades_it", async ({ page }) => {
  await signIn(page, ada);
  await openTimeline(page);
  const list = await openWarnings(page);
  // *Away* on Bob's issue: all four ways out, in the order the server offers them; *Delay* of
  // WEB-2: Move it after and Keep and accept.
  const away = warningAbout(list, people.shared);
  await expect(away).toContainText(bob.name);
  await expect(away.locator('[data-part="timeline-warning.ways"] button')).toHaveText(
    (["move-it-after", "make-it-longer", "give-to-someone-else", "keep-and-accept"] as const).map(
      (each) => t(`plan-way-out.${each}`),
    ),
  );
  await Promise.all(
    (["move-it-after", "give-to-someone-else", "keep-and-accept"] as const).map((each) =>
      expect(away.getByRole("button", way(each))).toBeVisible(),
    ),
  );
  const delay = warningAbout(list, delaySentence);
  await expect(delay.getByRole("button", way("move-it-after"))).toBeVisible();
  await expect(delay.getByRole("button", way("keep-and-accept"))).toBeVisible();
  await expect(delay.getByRole("button", way("give-to-someone-else"))).toHaveCount(0);
  // The list says how it treats people, last.
  await expect(list).toContainText(t("timeline.warnings-closing"));
  const before = await warningsButton(page).innerText();
  await warningAbout(list, people.holiday).getByRole("button", way("keep-and-accept")).click();
  // Accepted: faded at the end, with who accepted it, and no longer counted.
  const accepted = list.locator('[data-part="timeline-warning"][data-state~="accepted"]');
  await expect(accepted).toContainText(t("timeline.warning-accepted", { name: ada.name }));
  await expect(accepted.getByRole("button", way("keep-and-accept"))).toHaveCount(0);
  await expect(warningsButton(page)).not.toHaveText(before);
});

test("plan_13_a_preview_the_server_refuses_says_why_and_shows_nothing", async ({ page }) => {
  await signIn(page, ada);
  await openTimeline(page);
  const list = await openWarnings(page);
  await page.route(
    "**/api/v1/timeline/move-after",
    (route) =>
      route.fulfill({
        status: 404,
        contentType: "application/problem+json",
        body: JSON.stringify({
          type: "urn:bugaboo:error:resource.not-found",
          title: "resource.not-found",
          status: 404,
          code: "resource.not-found",
          message_key: "error.resource.not-found",
          params: {},
        }),
      }),
    { times: 1 },
  );
  const delay = warningAbout(list, delaySentence);
  await delay.getByRole("button", way("move-it-after")).click();
  await expect(page.getByText(t("error.resource.not-found"))).toBeVisible();
  await expect(delay.locator('[data-part="timeline-warning.preview"]')).toHaveCount(0);
  await expect(delay.getByRole("button", way("move-it-after"))).toBeEnabled();
});

test("plan_13_make_it_longer_saves_only_after_confirming_and_undo_puts_it_back", async ({
  page,
  server,
}) => {
  const before = await datesAs(server, ada, people.shared);
  await signIn(page, ada);
  await openTimeline(page);
  const list = await openWarnings(page);
  const away = warningAbout(list, people.shared);
  await away.getByRole("button", way("make-it-longer")).click();
  // The preview names the due date the issue needs; nothing is saved yet.
  const preview = away.locator('[data-part="timeline-warning.preview"]');
  await expect(preview).toContainText(keepingHyphens(people.shared));
  expect(await datesAs(server, ada, people.shared)).toEqual(before);
  await preview.getByRole("button", { name: t("timeline.change-due"), exact: true }).click();
  await expect
    .poll(async () => (await datesAs(server, ada, people.shared)).due)
    .not.toBe(before.due);
  const after = await datesAs(server, ada, people.shared);
  expect(after.start).toBe(before.start);
  expect(String(after.due).localeCompare(String(before.due))).toBe(1);
  // *Undo* puts the due date back.
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect.poll(() => datesAs(server, ada, people.shared)).toEqual(before);
});

test("plan_13_move_it_after_shows_a_preview_before_anything_moves", async ({ page, server }) => {
  const before = await datesAs(server, ada, planned.waits);
  await signIn(page, ada);
  await openTimeline(page);
  const list = await openWarnings(page);
  const delay = warningAbout(list, delaySentence);
  await delay.getByRole("button", way("move-it-after")).click();
  // The preview names what moves, and nothing has moved yet.
  const preview = delay.locator('[data-part="timeline-warning.preview"]');
  await expect(preview).toContainText(keepingHyphens(planned.waits));
  expect(await datesAs(server, ada, planned.waits)).toEqual(before);
  await preview.getByRole("button", { name: t("timeline.move-them", { count: 1 }) }).click();
  // WEB-2 now starts the day after WEB-1 ends.
  await expect
    .poll(async () => (await datesAs(server, ada, planned.waits)).start)
    .toBe(dayFromToday(4));
  await expect(page.getByText(t("timeline.moved-after", { count: 1 }))).toBeVisible();
});

test("plan_13_give_it_to_someone_else_suggests_nobody", async ({ page, server }) => {
  await signIn(page, ada);
  await openTimeline(page);
  const list = await openWarnings(page);
  await warningAbout(list, people.shared).getByRole("button", way("give-to-someone-else")).click();
  // The usual person picker, named for the way out: the people of the project by name, in the
  // order of their names, none marked and none first by rank.
  const picker = page.getByRole("dialog", { name: t("plan-way-out.give-to-someone-else") });
  await expect(picker).toBeVisible();
  await expect(picker.getByText(t("issues.assigned"), { exact: true })).toHaveCount(0);
  const give = new RegExp(`^${t("timeline.give-to-name", { name: "" })}`, "u");
  const names = await picker.getByRole("button", { name: give }).allInnerTexts();
  expect(names.length).toBeGreaterThan(1);
  await picker
    .getByRole("button", { name: t("timeline.give-to-name", { name: ada.name }) })
    .click();
  // Ada is now the person on the bar; the others assigned after the first stay.
  await expect.poll(() => firstAssigned(server, people.shared)).toBe(ada.name);
});
