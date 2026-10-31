import { mkdirSync } from "node:fs";
import { join } from "node:path";

import { type Locator, type Page } from "@playwright/test";

import { inOrder } from "./sequence.ts";
import { settingDefaults } from "@bugaboo/api-client";
import { expectLook } from "./checks.ts";

/** The folder the screenshots go to, from `BB_SCREENSHOT_DIR`; `undefined` when none are wanted. */
export const screenshotFolder = process.env["BB_SCREENSHOT_DIR"];

/** The folder of a milestone (or of a set of pictures) inside the screenshot folder, made if it is missing. */
export function folderOf(milestone: string): string {
  // No fallback: a picture is never written into the folder the tests run in.
  if (screenshotFolder === undefined) {
    throw new TypeError("Set BB_SCREENSHOT_DIR to take the screenshots.");
  }
  const target = join(screenshotFolder, milestone);
  mkdirSync(target, { recursive: true });
  return target;
}

/**
 * Whether a window or a question covers the page. It is pinned to the screen, so its picture is the
 * size of the screen: on a page grown to its content it would float in the middle of blank space.
 */
async function coversThePage(page: Page): Promise<boolean> {
  return (await page.locator("[role='dialog'], [role='alertdialog']").count()) > 0;
}

/**
 * Takes the pointer off the page, to a point outside the window, where there is no control to be
 * pointed at: the last control a test pressed (or a corner, where a *Back* may stand) would
 * otherwise show its hover fill in the picture.
 */
export async function pointerAway(page: Page): Promise<void> {
  await page.mouse.move(-10, -10);
}

/** The two looks the maintainer sees: the device asks for a light or a dark look, as a phone does. */
const looks = ["light", "dark"] as const;

/** The theme "Match my device" shows for each look, with the standard settings (the app's `data-look`). */
const themeOf = {
  light: settingDefaults.appearanceLightTheme,
  dark: settingDefaults.appearanceDarkTheme,
} as const;

/** The look the device does not ask for: asking for it first makes the app notice the change. */
const otherThan = { light: "dark", dark: "light" } as const;

/**
 * Takes the open screen in both looks, as `<name>-light.png` and `<name>-dark.png`, in the folder
 * of the milestone.
 */
export async function shoot(page: Page, name: string, milestone = "M1"): Promise<void> {
  await shootLooksInto(page, folderOf(milestone), name, "page");
}

/**
 * Takes the open screen in both looks as `<name>-light.png` and `<name>-dark.png` in `target`.
 * `shown: "page"` takes the whole page (a window or a question that covers the page is taken as
 * the screen shows it); `shown: "window"` takes what the window shows, as a person sees it.
 */
export async function shootLooksInto(
  page: Page,
  target: string,
  name: string,
  shown: "page" | "window",
): Promise<void> {
  await pointerAway(page);
  await inOrder(
    looks.map((look) => async () => {
      // Like a phone in dark mode: "Match my device" then shows the dark theme as chosen. The
      // other look comes first, because a page that already counts as light gets no change to
      // notice when it is asked for light again (the theme of a check before it would stay).
      await page.emulateMedia({ colorScheme: otherThan[look] });
      await page.emulateMedia({ colorScheme: look });
      await expectLook(page, themeOf[look]);
      await page.screenshot({
        path: join(target, `${name}-${look}.png`),
        fullPage: shown === "page" && !(await coversThePage(page)),
      });
    }),
  );
  await page.emulateMedia({ colorScheme: null });
}

/** Never taller than this, whatever a page asks for. */
const TALLEST = 12_000;

/** How much taller than the window the page is: the most any frame of it scrolls. */
function missingHeight(page: Page): Promise<number> {
  return page.evaluate(() => {
    const scrolling = document.scrollingElement;
    let most = 0;
    for (const element of document.querySelectorAll("*")) {
      const { overflowY: overflow, maxHeight } = getComputedStyle(element);
      // A list with a height of its own (`max-height`) scrolls inside its box whatever the window.
      const framed = element !== scrolling && maxHeight !== "none";
      if (!framed && (element === scrolling || overflow === "auto" || overflow === "scroll")) {
        most = Math.max(most, element.scrollHeight - element.clientHeight);
      }
    }
    // A sheet that covers the page does not scroll, it is cut off at the bottom of the window.
    for (const dialog of document.querySelectorAll("[role='dialog']")) {
      for (const inside of dialog.querySelectorAll("*")) {
        most = Math.max(
          most,
          Math.ceil(inside.getBoundingClientRect().bottom - window.innerHeight),
        );
      }
    }
    return most;
  });
}

/**
 * Makes the window as tall as the page needs, so that the whole page is in the picture: the app
 * scrolls inside its frame, where a full-page picture would cut it off. A taller window can make
 * the page taller still (a bottom bar moves), so it looks again, a few times. A list with a
 * height of its own (the people to put into a group) scrolls inside its box however tall the
 * window is, so it is not counted, and a look that finds no less missing than the one before
 * stops there.
 */
async function growToContent(page: Page, rounds = 4, before = Infinity): Promise<void> {
  const size = page.viewportSize();
  const missing = await missingHeight(page);
  if (
    rounds === 0 ||
    size === null ||
    missing <= 0 ||
    missing >= before ||
    size.height >= TALLEST
  ) {
    return;
  }
  await page.setViewportSize({
    width: size.width,
    height: Math.min(size.height + missing, TALLEST),
  });
  await page.waitForTimeout(SETTLING_MS);
  await growToContent(page, rounds - 1, missing);
}

/** How long the page takes to settle after the window changed. */
const SETTLING_MS = 300;

/**
 * Makes the window as tall as the page needs, lets `take` take the picture, and puts the window
 * back. Does nothing when no folder is wanted.
 */
async function withWholePage(page: Page, take: () => Promise<void>): Promise<void> {
  if (screenshotFolder === undefined) {
    return;
  }
  const original = page.viewportSize();
  if (!(await coversThePage(page))) {
    await growToContent(page);
  }
  await take();
  if (original !== null) {
    await page.setViewportSize(original);
  }
}

/**
 * Takes the whole open screen in both looks, as `<name>-light.png` and `<name>-dark.png` in the
 * folder of the milestone (`M2/demo`), with the window grown to the height of the page. Does
 * nothing when no folder is wanted.
 */
export function shootWhole(page: Page, name: string, milestone: string): Promise<void> {
  return withWholePage(page, () => shoot(page, name, milestone));
}

/**
 * Takes the whole open screen in the theme it shows, as `<name>-<theme>.png` in the folder of the
 * milestone, with the window grown to the height of the page. The page must show that theme: the
 * picture is not taken otherwise. Does nothing when no folder is wanted.
 */
export function shootWholeAs(
  page: Page,
  name: string,
  milestone: string,
  theme: string,
  mask: readonly Locator[] = [],
): Promise<void> {
  return withWholePage(page, async () => {
    await pointerAway(page);
    await expectLook(page, theme);
    await page.screenshot({
      path: join(folderOf(milestone), `${name}-${theme}.png`),
      fullPage: !(await coversThePage(page)),
      mask: [...mask],
    });
  });
}

/**
 * Takes the window as it is in the theme it shows, as `<name>-<theme>.png` in the folder of the
 * milestone, without growing it to the page: for a list that reads more rows when it is made taller.
 * Does nothing when no folder is wanted, like every picture of a demo.
 */
export async function shootWindowAs(
  page: Page,
  name: string,
  milestone: string,
  theme: string,
): Promise<void> {
  if (screenshotFolder === undefined) {
    return;
  }
  await pointerAway(page);
  await expectLook(page, theme);
  await page.screenshot({ path: join(folderOf(milestone), `${name}-${theme}.png`) });
}

/** How long a sheet, a menu or a list takes to settle; the picture is taken when it has stopped. */
const SETTLING_FOR_SHOT_MS = 500;

/** Waits for the screen to settle, then takes it in both looks. */
export async function shot(page: Page, name: string, milestone: string): Promise<void> {
  await page.waitForTimeout(SETTLING_FOR_SHOT_MS);
  await shoot(page, name, milestone);
}

/** Waits for the screen to settle, then takes the whole of it in the theme it shows. */
export async function shotWholeAs(
  page: Page,
  name: string,
  milestone: string,
  theme: string,
  mask: readonly Locator[] = [],
): Promise<void> {
  await page.waitForTimeout(SETTLING_FOR_SHOT_MS);
  await shootWholeAs(page, name, milestone, theme, mask);
}

/** The same with the window grown to the height of the page, then put back. */
export async function shotWhole(page: Page, name: string, milestone: string): Promise<void> {
  await page.waitForTimeout(SETTLING_FOR_SHOT_MS);
  await shootWhole(page, name, milestone);
}
