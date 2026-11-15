import type { Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { gapUnder } from "./support/geometry.ts";
import { handleIn, slotsIn } from "./support/layout.ts";
import { dueDate, openLayoutOfBugs, prepareLayouts, setLayoutOfWeb } from "./support/layouts.ts";
import { carryOverHead, letGoOn } from "./support/hand-drag.ts";
import { SETTINGS_TOPIC } from "./support/pane-head.ts";

// A drag over the folded head of a pane scrolls the box under it, as a drag near the top edge of a box does: the head
// stands where the box's top edge is (UX §6.5). The pointer moves as a hand moves it, never by `dragTo`.

test.use({ preparation: { run: prepareLayouts } });

/** The part of the head of a pane. */
const HEAD_PART = "pane-head";

/** A window high enough to show the first slot under the whole head, and lower than the editor. */
const WINDOW = { width: 1280, height: 720 } as const;

/** Opens the editor of the layout of *Bug*, with a layout of its own and *Review deadline* not in it. */
async function openEditor(page: Page): Promise<void> {
  await page.setViewportSize(WINDOW);
  await signIn(page, ada);
  await openLayoutOfBugs(page);
  await page
    .getByRole("button", { name: t("layouts.customize-project", { project: "Website" }) })
    .click();
}

test("ux_06 dragging a field over the folded head scrolls the topic under it to a slot it covered", async ({
  page,
  server,
}) => {
  await setLayoutOfWeb(server, "inherited");
  await openEditor(page);
  const section = page.getByRole("region", {
    name: t("layouts.section", { number: "1" }),
    exact: true,
  });
  const available = page.getByRole("list", { name: t("layouts.available"), exact: true });
  const handle = handleIn(available, t("layouts.field-drag", { name: dueDate }));
  await expect(handle).toBeVisible();

  // The topic at its end: the head is folded, and the first slot of the first section is under it or above it.
  const topic = page.locator(SETTINGS_TOPIC);
  await topic.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  const slot = slotsIn(section).first();
  await expect.poll(() => gapUnder(slot, HEAD_PART)).toBeLessThan(0);
  await handle.scrollIntoViewIfNeeded();

  // The field is carried over the head and held there: the topic scrolls under it to its top, by itself.
  await carryOverHead(page, handle);
  await expect.poll(() => topic.evaluate((node) => node.scrollTop)).toBe(0);

  // The slot now stands under the whole head, in the window, and the field is let go on it.
  await expect.poll(() => gapUnder(slot, HEAD_PART)).toBeGreaterThanOrEqual(0);
  await expect(slot).toBeInViewport();
  await letGoOn(page, slot);
  await expect(available.getByText(dueDate, { exact: true })).toHaveCount(0);
  await expect(section.getByText(dueDate, { exact: true })).toBeVisible();
});
