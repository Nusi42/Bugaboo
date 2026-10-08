import { mkdirSync } from "node:fs";
import { join } from "node:path";

import { expect, type Page } from "@playwright/test";

import { inOrder } from "./sequence.ts";

/** The folder the screenshots go to, from `BB_SCREENSHOT_DIR`; `undefined` when none are wanted. */
export const screenshotFolder = process.env["BB_SCREENSHOT_DIR"];

/** The two looks the maintainer sees: the device asks for a light or a dark look, as a phone does. */
const looks = ["light", "dark"] as const;

/** The theme "Match my device" shows for each look, with the standard settings (the app's `data-look`). */
const themeOf = { light: "daylight", dark: "dusk" } as const;

/** The look the device does not ask for: asking for it first makes the app notice the change. */
const otherThan = { light: "dark", dark: "light" } as const;

/**
 * Takes the open screen in both looks, as `<name>-light.png` and `<name>-dark.png`, in the folder
 * of the milestone.
 */
export async function shoot(page: Page, name: string, milestone = "M1"): Promise<void> {
  const target = join(screenshotFolder ?? ".", milestone);
  mkdirSync(target, { recursive: true });
  await inOrder(
    looks.map((look) => async () => {
      // Like a phone in dark mode: "Match my device" then shows the dark theme as chosen. The
      // other look comes first, because a page that already counts as light gets no change to
      // notice when it is asked for light again (the theme of a check before it would stay).
      await page.emulateMedia({ colorScheme: otherThan[look] });
      await page.emulateMedia({ colorScheme: look });
      await expect(page.locator("html")).toHaveAttribute("data-look", themeOf[look]);
      await page.screenshot({ path: join(target, `${name}-${look}.png`), fullPage: true });
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
      const overflow = getComputedStyle(element).overflowY;
      if (element === scrolling || overflow === "auto" || overflow === "scroll") {
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
 * the page taller still (a bottom bar moves), so it looks again, a few times.
 */
async function growToContent(page: Page, rounds = 4): Promise<void> {
  const size = page.viewportSize();
  const missing = await missingHeight(page);
  if (rounds === 0 || size === null || missing <= 0 || size.height >= TALLEST) {
    return;
  }
  await page.setViewportSize({
    width: size.width,
    height: Math.min(size.height + missing, TALLEST),
  });
  await page.waitForTimeout(SETTLING_MS);
  await growToContent(page, rounds - 1);
}

/** How long the page takes to settle after the window changed. */
const SETTLING_MS = 300;

/**
 * Takes the whole open screen in both looks, as `<name>-light.png` and `<name>-dark.png` in the
 * folder of the milestone (`M2/demo`), with the window grown to the height of the page. Does
 * nothing when no folder is wanted.
 */
export async function shootWhole(page: Page, name: string, milestone: string): Promise<void> {
  if (screenshotFolder === undefined) {
    return;
  }
  const original = page.viewportSize();
  await growToContent(page);
  await shoot(page, name, milestone);
  if (original !== null) {
    await page.setViewportSize(original);
  }
}
