import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { part } from "../../../packages/ui/src/tokens.ts";
import { keyboardStep, menuStep, windowEdges } from "../../../packages/ui/src/window-size.ts";
import { partIn } from "./parts.ts";
import { newIssueDialog, openNewIssueDialog, titleLabel } from "./issues.ts";

export { keyboardStep, menuStep, windowEdges };

/** The box of the window: the part that the size variables are set on. */
export function windowBox(page: Page): Locator {
  return partIn(page, part.overlayModal, "window");
}

/** The bar a minimized window becomes. */
export function windowDock(page: Page): Locator {
  return partIn(page, part.windowDock);
}

/** The button of the title bar of the open window. */
export function windowButton(page: Page, name: string): Locator {
  return newIssueDialog(page).getByRole("button", { name, exact: true });
}

/** How big the window is on the page right now. */
export async function windowSize(page: Page): Promise<{ width: number; height: number }> {
  const box = await boxOf(windowBox(page));
  return { width: Math.round(box.width), height: Math.round(box.height) };
}

/** Opens the new-issue window and waits until its fields are there, so its size is settled. */
export async function openWindowReady(page: Page): Promise<void> {
  await openNewIssueDialog(page);
  await expect(newIssueDialog(page).getByLabel(titleLabel(), { exact: true })).toBeVisible();
}

/** The title box of the window. */
export function titleBox(page: Page): Locator {
  return newIssueDialog(page).getByLabel(titleLabel(), { exact: true });
}

/** Presses the size keys: Ctrl and Alt with an arrow key, inside the window. */
export async function pressSizeKey(page: Page, arrow: string): Promise<void> {
  await titleBox(page).focus();
  await page.keyboard.press(`Control+Alt+${arrow}`);
}

/** Opens the size menu of the title bar and picks a step. */
export async function pickSizeStep(page: Page, step: "wider" | "narrower" | "taller" | "shorter") {
  await windowButton(page, t("window.size-menu")).click();
  await page.getByRole("button", { name: t(`window.${step}`), exact: true }).click();
}

/** Where the text of an element starts, from the left of the page: after its own padding and border. */
export function textStartOf(element: Locator): Promise<number> {
  return element.evaluate((node) => {
    const range = document.createRange();
    range.selectNodeContents(node);
    return range.getBoundingClientRect().left;
  });
}

/** Where an element is on the page; it must be there. */
export async function boxOf(
  element: Locator,
): Promise<{ x: number; y: number; width: number; height: number }> {
  const box = await element.boundingBox();
  if (box === null) {
    throw new TypeError("The element is not on the page.");
  }
  return box;
}

/** The text column and the column of the fields of the form, and the window, as boxes on the page. */
export async function columns(page: Page) {
  return {
    text: await boxOf(partIn(page, part.windowLayoutText)),
    fields: await boxOf(partIn(page, part.windowLayoutFields)),
    modal: await boxOf(windowBox(page)),
  };
}

/** Scrolls the form of the window to its end. */
export async function scrollFormToEnd(page: Page): Promise<void> {
  await page.evaluate((selector) => {
    const body = document.querySelector(selector);
    body?.scrollTo({ top: body.scrollHeight });
  }, `[data-part="${part.windowBody}"]`);
}

/** The size of the page as the window can fill it. */
export function pageSize(page: Page): { width: number; height: number } {
  const size = page.viewportSize();
  if (size === null) {
    throw new TypeError("The page has no size.");
  }
  return size;
}

/** Drags the east edge of the window by `by` pixels with the mouse; the window grows twice as much, centered. */
export async function dragEastEdge(page: Page, by: number): Promise<void> {
  const box = await boxOf(partIn(page, "window.handle", "e"));
  const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + by, y, { steps: 6 });
  await page.mouse.up();
}

/** The buttons of the footer of the new-issue window, as boxes: Cancel, Create and start another, Create. */
export async function footerButtons(page: Page) {
  const named = (name: string) =>
    boxOf(newIssueDialog(page).getByRole("button", { name, exact: true }));
  return {
    note: await boxOf(partIn(page, "window.footer-note")),
    cancel: await named(t("common.cancel")),
    another: await named(t("issues.create-another")),
    create: await named(t("issues.create")),
  };
}

/** The box lies inside the screen: a window or a question is pinned to it and never floats in a taller page. */
export async function expectInsideTheScreen(page: Page, box: Locator): Promise<void> {
  const [inside, screen] = [await boxOf(box), pageSize(page)];
  expect(inside.x).toBeGreaterThanOrEqual(0);
  expect(inside.y).toBeGreaterThanOrEqual(0);
  expect(inside.x + inside.width).toBeLessThanOrEqual(screen.width);
  expect(inside.y + inside.height).toBeLessThanOrEqual(screen.height);
}

/**
 * How many characters of the text of an element stand on each line, from the first line to the last:
 * the characters are grouped by the top of their box, so a name inside the element counts too.
 */
export function lineLengths(element: Locator): Promise<readonly number[]> {
  return element.evaluate((node) => {
    const range = document.createRange();
    const counts = new Map<number, number>();
    const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
    for (let text = walker.nextNode(); text !== null; text = walker.nextNode()) {
      for (let at = 0; at < (text.textContent?.length ?? 0); at += 1) {
        range.setStart(text, at);
        range.setEnd(text, at + 1);
        const box = range.getBoundingClientRect();
        // A space that the line break swallowed has no box.
        if (box.width > 0 || box.height > 0) {
          const top = Math.round(box.top);
          counts.set(top, (counts.get(top) ?? 0) + 1);
        }
      }
    }
    return [...counts.keys()].toSorted((a, b) => a - b).map((top) => counts.get(top) ?? 0);
  });
}

/** How many lines the words of an element take: the lines of [`lineLengths`]. */
export async function linesOf(element: Locator): Promise<number> {
  return (await lineLengths(element)).length;
}

/** Holds back the first request that keeps the sizes, so a second one is sent before it is done. */
export async function holdFirstSizeSave(page: Page, ms: number): Promise<void> {
  const held = { already: false };
  await page.route("**/api/v1/me/settings/window.sizes", async (route) => {
    const hold = route.request().method() === "PUT" && !held.already;
    held.already = held.already || hold;
    await Promise.all(
      hold
        ? [
            new Promise((resolve) => {
              setTimeout(resolve, ms);
            }),
          ]
        : [],
    );
    await route.continue();
  });
}

/**
 * Waits until the server has answered the requests that keep the sizes, as many as `count`: a
 * reload before that would ask for a size that is not saved yet.
 */
export async function sizesSaved(page: Page, count: number, act: () => Promise<void>) {
  let answered = 0;
  const done = new Promise<void>((resolve) => {
    page.on("response", (response) => {
      if (response.request().method() === "PUT" && response.url().includes("window.sizes")) {
        answered += 1;
        if (answered === count) {
          resolve();
        }
      }
    });
  });
  await act();
  await done;
}
