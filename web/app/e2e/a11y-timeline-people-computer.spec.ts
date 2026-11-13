import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { openTimeline, pressOf } from "./support/timeline.ts";
import { people, prepareTimelinePeople, warningsButton } from "./support/timeline-people.ts";

// The accessibility suite for the people and the warnings on the timeline (TEST-A11Y, UX §4.7): the
// list of plan warnings, reached and used with the keyboard only, and a bar with its person, its
// hatched days and its card, in the four themes.

test.use({ preparation: { run: prepareTimelinePeople } });

test("a11y the list of plan warnings and its ways out, with the keyboard only", async ({
  page,
}) => {
  await signIn(page, ada);
  await openTimeline(page);
  await warningsButton(page).focus();
  await page.keyboard.press("Enter");
  await expect(warningsButton(page)).toHaveAttribute("aria-expanded", "true");
  const list = page.getByRole("region", { name: t("timeline.warnings-title") });
  await expect(list).toBeVisible();
  await expectGoodInEveryLook(page);
  // A way out is described by the sentence of its warning; *Move it after* shows its preview.
  const move = list
    .getByRole("button", { name: t("plan-way-out.move-it-after"), exact: true })
    .first();
  await expect(move).toHaveAccessibleDescription(/.+/u);
  await move.focus();
  await page.keyboard.press("Enter");
  await expect(list.locator('[data-part="timeline-warning.preview"]')).toBeVisible();
  await expectGoodInEveryLook(page);
  await list.getByRole("button", { name: t("common.cancel"), exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(list.locator('[data-part="timeline-warning.preview"]')).toHaveCount(0);
});

test("a11y a bar with its person, its hatched days and its card", async ({ page }) => {
  await signIn(page, ada);
  await openTimeline(page);
  const bar = pressOf(page, people.shared);
  await expect(bar).toHaveAccessibleName(new RegExp(t("timeline.away"), "u"));
  await bar.focus();
  await page.keyboard.press("Enter");
  const card = page.getByRole("region", { name: t("timeline.card", { key: people.shared }) });
  await expect(card).toBeVisible();
  await expectGoodInEveryLook(page);
});
