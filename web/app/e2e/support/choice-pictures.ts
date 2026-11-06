import { join } from "node:path";

import { type Locator, type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { folderOf, pointerAway } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";
import { showTheme } from "./showcase.ts";

/** How tall the window is while a picture of a part of the screen is taken. */
const PICTURE_HEIGHT = 2400;

/**
 * Takes what `region` shows (a part of the screen, not all of it) in the four themes, each chosen
 * in *My settings › Appearance* before `open` opens the screen: `<name>-<layout>-<theme>.png` in
 * the folder `milestone` below the one the run was given. The pictures wait until a window that
 * opens has finished fading in, so the page behind never shows through it.
 */
export async function inFourThemesOf(
  page: Page,
  region: () => Locator,
  shot: {
    readonly name: string;
    readonly milestone: string;
    readonly asIs?: boolean;
    /** The picture ends under the last thing the region shows, not at the end of a tall card. */
    readonly toContent?: boolean;
  },
  open: () => Promise<void>,
): Promise<void> {
  const layout = layoutOf(page);
  const window = page.viewportSize();
  await inOrder(
    themes.map((theme) => async () => {
      // A tall window, so what the picture shows is never under the bottom bar of a phone.
      if (shot.asIs !== true) {
        await page.setViewportSize({
          width: window?.width ?? 1280,
          height: PICTURE_HEIGHT,
        });
      }
      await showTheme(page, theme, paths.appearance);
      await open();
      await page.waitForFunction(() => document.querySelector("[data-entering]") === null);
      await pointerAway(page);
      const path = join(folderOf(shot.milestone), `${shot.name}-${layout}-${theme.id}.png`);
      if (shot.toContent === true) {
        await page.screenshot({ path, fullPage: true, clip: await contentOf(region()) });
      } else {
        await region().screenshot({ path });
      }
      if (window !== null && shot.asIs !== true) {
        await page.setViewportSize(window);
      }
    }),
  );
}

/** The room around the last thing of a picture that ends at its content, in pixels. */
const CONTENT_MARGIN = 16;

/**
 * The part of the region that holds what it shows: from its top to the bottom of its lowest element
 * that has no element inside (a text, a box, a picture) and a little room under it, so a card that
 * is taller than what is in it is not drawn empty.
 */
async function contentOf(region: Locator): Promise<{
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}> {
  const box = await region.boundingBox();
  if (box === null) {
    throw new Error("The region of the picture is not on the screen");
  }
  const lowest = await region.evaluate((element) => {
    let bottom = 0;
    for (const each of element.querySelectorAll("*")) {
      const area = each.getBoundingClientRect();
      if (each.childElementCount === 0 && area.width > 0 && area.height > 0) {
        bottom = Math.max(bottom, area.bottom);
      }
    }
    return bottom;
  });
  const height = Math.min(box.height, lowest - box.y + CONTENT_MARGIN);
  return { ...box, height };
}

/**
 * Shows the lists of the boxes of the window that is open as the browser lists their options, one
 * under the other in their order: a browser draws the open list of a box itself, outside the page,
 * where a picture cannot see it.
 */
export async function listTheOptions(page: Page): Promise<void> {
  await page.evaluate(() => {
    for (const box of document.querySelectorAll("[role='dialog'] select")) {
      if (box instanceof HTMLSelectElement) {
        box.size = box.options.length;
        box.style.height = "auto";
      }
    }
  });
}

/**
 * Opens the window that changes the first filter of the search: on a phone the filters are in a
 * sheet that the button *Filters* opens first.
 */
export async function openTheFilter(page: Page): Promise<void> {
  if (layoutOf(page) === "phone") {
    await page.getByRole("button", { name: /^Filters/u }).click();
  }
  await page
    .getByRole("button", { name: /^Change the filter/u })
    .first()
    .click();
  await page.locator("[data-part='overlay.modal']").last().waitFor();
}
