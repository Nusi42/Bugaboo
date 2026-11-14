import type { Locator, Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { issueAddress } from "../src/shared/paths.ts";

// On a computer an open issue scrolls in boxes of its own (UX §6.5): the sections and the Details.
// The next issue starts both at the top, and Back brings each to where it was.

test.use({ preparation: { run: prepareWeb } });

/** A low window, so the sections of an issue are higher than their box. */
const LOW_WINDOW = { width: 1280, height: 420 } as const;

/** The box of the sections of the open issue. */
function sectionsOf(page: Page): Locator {
  return page.locator("[data-part='issue-body'][data-variant~='split'] [data-part='issue-main']");
}

/** Scrolls a box down a little and says where it stands then. */
async function scrolled(box: Locator): Promise<number> {
  await expect(box).toBeVisible();
  await box.evaluate((element) => {
    element.scrollBy(0, 200);
  });
  await expect.poll(() => box.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
  return box.evaluate((element) => element.scrollTop);
}

/** Goes to another address in the app the way a link does, without loading the page again. */
async function follow(page: Page, address: string): Promise<void> {
  await page.evaluate((to) => {
    history.pushState(null, "", to);
    dispatchEvent(new PopStateEvent("popstate"));
  }, address);
}

/** The sides of its box that cut the focus ring of a control in the sections of an issue. */
function ringCutAt(control: Locator): Promise<string[]> {
  return control.evaluate((element) => {
    const box = element.closest("[data-part='issue-main']") ?? document.body;
    const style = getComputedStyle(element);
    // Both are lengths in px as the browser computes them.
    const ring =
      Number(style.outlineWidth.replace("px", "")) + Number(style.outlineOffset.replace("px", ""));
    const own = element.getBoundingClientRect();
    const start = box.getBoundingClientRect().left + box.clientLeft;
    const sides: string[] = [];
    if (own.left - ring < start) {
      sides.push("start");
    }
    if (own.right + ring > start + box.clientWidth) {
      sides.push("end");
    }
    return sides;
  });
}

test("ux_06 another issue starts its sections at the top, and Back brings them to where they were", async ({
  page,
}) => {
  await page.setViewportSize(LOW_WINDOW);
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const sections = sectionsOf(page);
  await expect(sections).toBeVisible();
  const was = await scrolled(sections);
  await follow(page, issueAddress("WEB-2"));
  await expect(page).toHaveURL(/WEB-2$/u);
  await expect.poll(() => sections.evaluate((element) => element.scrollTop)).toBe(0);
  await page.goBack();
  await expect(page).toHaveURL(/WEB-1$/u);
  await expect.poll(() => sections.evaluate((element) => element.scrollTop)).toBe(was);
});

test("ux_06 the focus ring of a control at the edge of a box that scrolls is not cut", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const sections = sectionsOf(page);
  const first = sections.locator("button, a[href], input, textarea, select").first();
  await expect(first).toBeVisible();
  // With the keyboard, so the ring shows (`:focus-visible`).
  await first.focus();
  await page.keyboard.press("Shift+Tab");
  await page.keyboard.press("Tab");
  await expect(first).toBeFocused();
  const cut = await ringCutAt(first);
  expect(cut, "the edges where the ring is cut").toEqual([]);
});
