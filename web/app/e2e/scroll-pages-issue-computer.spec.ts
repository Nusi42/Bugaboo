import type { Locator, Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

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

/** Opens another issue as a person does: by its row in the slim list beside the open one. */
async function openFromSlimList(page: Page, key: string): Promise<void> {
  await page
    .locator(`[data-part~='focus-view.list'] a[href$='${issueAddress(key)}']`)
    .first()
    .click();
}

/** The sides of the box it stands in (the sections of an issue, unless named) that cut the focus ring of a control. */
function ringCutAt(control: Locator, inside = "issue-main"): Promise<string[]> {
  return control.evaluate((element, part) => {
    const box = element.closest(`[data-part='${part}']`) ?? document.body;
    const style = getComputedStyle(element);
    // Both are lengths in px as the browser computes them.
    const ring =
      Number(style.outlineWidth.replace("px", "")) + Number(style.outlineOffset.replace("px", ""));
    const own = element.getBoundingClientRect();
    const frame = box.getBoundingClientRect();
    const start = frame.left + box.clientLeft;
    const top = frame.top + box.clientTop;
    const sides: string[] = [];
    if (own.left - ring < start) {
      sides.push("start");
    }
    if (own.right + ring > start + box.clientWidth) {
      sides.push("end");
    }
    if (own.top - ring < top) {
      sides.push("top");
    }
    if (own.bottom + ring > top + box.clientHeight) {
      sides.push("bottom");
    }
    return sides;
  }, inside);
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
  await openFromSlimList(page, "WEB-2");
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
  expect(await sections.evaluate((element) => element.scrollTop)).toBe(0);
  const cut = await ringCutAt(first);
  expect(cut, "the edges where the ring is cut").toEqual([]);
  // The last control, with the box scrolled to its end.
  const last = sections.locator("button, a[href], input, textarea, select").last();
  await last.focus();
  await sections.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect(last).toBeFocused();
  expect(await ringCutAt(last), "the edges where the ring of the last control is cut").toEqual([]);
});

test("ux_06 j opens the next issue of the slim list, and its sections start at the top", async ({
  page,
}) => {
  await page.setViewportSize(LOW_WINDOW);
  await signIn(page, ada);
  // The slim list has the newest first: WEB-1, the oldest, comes after WEB-2.
  await page.goto(issueAddress("WEB-2"));
  const sections = sectionsOf(page);
  await scrolled(sections);
  await page.keyboard.press("j");
  await expect(page).toHaveURL(/WEB-1$/u);
  await expect.poll(() => sections.evaluate((element) => element.scrollTop)).toBe(0);
});

test("ux_3_4 Previous is off at the first issue of the slim list and Next at the last", async ({
  page,
}) => {
  await signIn(page, ada);
  const previous = page.getByRole("button", { name: t("list.previous"), exact: true });
  const next = page.getByRole("button", { name: t("list.next"), exact: true });
  // The slim list has the newest first: WEB-3 is the first, WEB-1 the last.
  await page.goto(issueAddress("WEB-3"));
  await expect(previous).toBeDisabled();
  await expect(next).toBeEnabled();
  await page.goto(issueAddress("WEB-2"));
  await expect(previous).toBeEnabled();
  await expect(next).toBeEnabled();
  await page.goto(issueAddress("WEB-1"));
  await expect(next).toBeDisabled();
  await expect(previous).toBeEnabled();
});

test("ux_06 the focus ring of a control in the head over the boxes of an issue is not cut", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const controls = page.locator(
    "[data-part='pane-head'] :is(button, a[href], input, textarea, select)",
  );
  const [first, last] = [controls.first(), controls.last()];
  await expect(first).toBeVisible();
  // With the keyboard, so the ring shows (`:focus-visible`).
  await last.focus();
  await page.keyboard.press("Shift+Tab");
  await page.keyboard.press("Tab");
  await expect(last).toBeFocused();
  expect(await ringCutAt(last, "pane-head"), "the edges where the ring is cut").toEqual([]);
  await first.focus();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Shift+Tab");
  await expect(first).toBeFocused();
  expect(await ringCutAt(first, "pane-head"), "the edges where the ring is cut").toEqual([]);
});
