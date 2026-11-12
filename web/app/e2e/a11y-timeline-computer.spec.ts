import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { openPlan, planned, prepareTimeline, pressOf } from "./support/timeline.ts";

// The accessibility suite for the timeline on a computer (TEST-A11Y, UX §4.7): the chart, the card
// of a selected bar and the window *Set dates…*, each reached with the keyboard only, in the four
// themes. Every drag of the timeline has its key; the tests of `timeline-computer.spec.ts` use them.

test.use({ preparation: { run: prepareTimeline } });

test("a11y the timeline and the card of a bar, with the keyboard only", async ({ page }) => {
  await signIn(page, ada);
  await openPlan(page);
  await expectGoodInEveryLook(page);
  const bar = pressOf(page, planned.first);
  await bar.focus();
  await page.keyboard.press("Enter");
  await expect(bar).toHaveAttribute("aria-pressed", "true");
  const card = page.getByRole("region", { name: t("timeline.card", { key: planned.first }) });
  await expect(card).toBeVisible();
  await expectGoodInEveryLook(page);
  await card.getByRole("button", { name: t("timeline.set-dates"), exact: true }).focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", {
    name: t("timeline.set-dates-title", { key: planned.first }),
  });
  await expect(window).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
});
