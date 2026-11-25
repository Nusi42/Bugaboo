import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { expectStill, landmarkOf, movementWhile } from "./support/movement.ts";
import { barOf, goalTitle, openPlan, planned, pressOf } from "./support/timeline.ts";
import { prepareDrivers, release } from "./support/timeline-drivers.ts";

// Deadline drivers on a computer (PLAN-05, UX §4.7, ADR-0043 amendment of 2026-11-24): the switch
// outlines the chain that sets the end of the goal, the card says what each bar sets, the outline
// is a pattern under forced colors and words for screen readers, and switching moves no bar. The
// release is a line with its diamond and name on its *Release date*.

test.use({ preparation: { run: prepareDrivers } });

/** Switches *Deadline drivers* of the tools on, by its words, as a person presses it. */
async function switchDriversOn(page: Page): Promise<void> {
  await page.getByText(t("timeline.deadline-drivers"), { exact: true }).click();
  await expect(page.getByRole("switch", { name: t("timeline.deadline-drivers") })).toBeChecked();
}

/** What a driver of the goal says it sets. */
const setsGoal = t("timeline.sets-end", { end: goalTitle });

test("plan_05_the_switch_outlines_the_drivers_and_the_card_says_what_they_set", async ({
  page,
}) => {
  await signIn(page, ada);
  await openPlan(page);
  // The release stands on its day with its name.
  await expect(page.locator('[data-part="timeline-release"]')).toContainText(release.name);
  const first = barOf(page, planned.first);
  await expect(first).not.toHaveAttribute("data-state", /\bdriver\b/u);
  // Switching it on moves no bar: the outline is drawn around the bars where they stand.
  const marks = [
    await landmarkOf(first, "first bar"),
    await landmarkOf(barOf(page, planned.waits), "waiting bar"),
  ];
  const switching = await movementWhile(page, marks, async () => {
    await switchDriversOn(page);
    await expect(first).toHaveAttribute("data-state", /\bdriver\b/u);
  });
  expectStill(switching);
  await expect(barOf(page, planned.waits)).toHaveAttribute("data-state", /\bdriver\b/u);
  // An issue that sets no end is not outlined.
  await expect(barOf(page, planned.dragged)).not.toHaveAttribute("data-state", /\bdriver\b/u);
  // The card of a driver says which end it sets.
  await pressOf(page, planned.first).click();
  const card = page.getByRole("region", { name: t("timeline.card", { key: planned.first }) });
  await expect(card).toContainText(setsGoal);
  // The switch is part of the view: the address keeps it.
  await expect(page).toHaveURL(/drivers=(%22)?1/u);
});

test("plan_05_switching_deadline_drivers_on_moves_nothing_in_a_narrow_window", async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 900 });
  await signIn(page, ada);
  await openPlan(page);
  const first = barOf(page, planned.first);
  const marks = [
    await landmarkOf(first, "first bar"),
    await landmarkOf(barOf(page, planned.dragged), "a bar below"),
  ];
  const switching = await movementWhile(page, marks, async () => {
    await switchDriversOn(page);
    await expect(first).toHaveAttribute("data-state", /\bdriver\b/u);
  });
  expectStill(switching);
});

test("plan_05_the_outline_is_seen_in_forced_colours_and_read_by_a_screen_reader", async ({
  page,
}) => {
  await signIn(page, ada);
  await openPlan(page);
  await switchDriversOn(page);
  const first = barOf(page, planned.first);
  await expect(first).toHaveAttribute("data-state", /\bdriver\b/u);
  // The name of the bar says it in words.
  await expect(pressOf(page, planned.first)).toHaveAccessibleName(new RegExp(setsGoal, "u"));
  // Under forced colors the outline is a pattern, not only a color.
  await page.emulateMedia({ forcedColors: "active" });
  await expect
    .poll(() => first.evaluate((bar) => getComputedStyle(bar).outlineStyle))
    .toBe("dashed");
  // The diamond of the release keeps an edge where the fill is gone.
  const diamond = page.locator('[data-part="timeline-release.diamond"]').first();
  await expect
    .poll(() => diamond.evaluate((mark) => getComputedStyle(mark).borderTopStyle))
    .toBe("solid");
});

test("a11y the timeline with deadline drivers on, in the four themes", async ({ page }) => {
  await signIn(page, ada);
  await openPlan(page);
  await switchDriversOn(page);
  await expect(barOf(page, planned.first)).toHaveAttribute("data-state", /\bdriver\b/u);
  await expectGoodInEveryLook(page);
});
