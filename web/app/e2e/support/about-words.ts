import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { downloadMessages, OFFERED } from "../../src/features/about/index.ts";
import { pageSize } from "../../src/shared/paged.ts";
import { type ApiSession } from "./api.ts";
import { signedInAdmin } from "./access.ts";
import { drawn } from "./frames.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

/** What About says of this build, as the server states it. */
export type About = Schema<"AboutBody">;

/** What the server says About shows, read as the administrator. */
export async function aboutOf(server: Instance): Promise<About> {
  const admin = await signedInAdmin(server);
  const about = dataOf(await admin.client.GET("/api/v1/about"));
  await admin.dispose();
  return about;
}

/** The name of the last part of the list, read through every page the server gives. */
export async function lastPartName(server: Instance): Promise<string> {
  const admin = await signedInAdmin(server);
  const last = await lastPartFrom(admin, undefined, "");
  await admin.dispose();
  return last;
}

/** The last part from the page at `cursor` on, by following the cursors. */
async function lastPartFrom(
  admin: ApiSession,
  cursor: string | undefined,
  before: string,
): Promise<string> {
  const page = dataOf(
    await admin.client.GET("/api/v1/about/parts", {
      params: {
        query: { limit: 100, ...(cursor === undefined ? {} : { cursor }) },
      },
    }),
  );
  const name = page.items.at(-1)?.name ?? before;
  return page.next_cursor === null || page.next_cursor === undefined
    ? name
    : lastPartFrom(admin, page.next_cursor, name);
}

/** The address of a page of the parts. */
const PARTS_ADDRESS = "/api/v1/about/parts";

/**
 * Reads every page after the first by pressing *Show more parts*, once for each page the API gives (each press waits
 * for the answer of its page), checks that the button is gone, then takes the last row into view with the keyboard
 * (the rows are drawn only where they are in the window) and checks that it is the last part.
 */
export async function readPartsToTheEnd(
  page: Page,
  list: Locator,
  about: About,
  last: string,
): Promise<void> {
  const more = morePartsButton(page);
  const later = laterPages(about);
  expect(later, "the parts take more than one page").toBeGreaterThan(0);
  await inOrder(
    Array.from({ length: later }, () => async () => {
      await pressedForAPage(page, () => more.click());
    }),
  );
  await expect(more).toHaveCount(0);
  await list.focus();
  await page.keyboard.press("End");
  await expect(list.getByText(last, { exact: true })).toBeVisible();
}

/** The button that reads the next page of the parts. */
export function morePartsButton(page: Page): Locator {
  return page.getByRole("button", { name: t("about.parts-more"), exact: true });
}

/** Does `press` to *Show more parts* and waits for the answer of the page it reads. */
async function pressedForAPage(page: Page, press: () => Promise<void>): Promise<void> {
  const answered = page.waitForResponse(
    (answer) => answer.url().includes(PARTS_ADDRESS) && answer.request().method() === "GET",
  );
  await press();
  await answered;
}

/** How far the box that scrolls around `inside` is scrolled down, or the page when no box around it scrolls. */
function scrolledAround(inside: Locator): Promise<number> {
  return inside.evaluate((element) => {
    for (let box = element.parentElement; box !== null; box = box.parentElement) {
      const { overflowY } = getComputedStyle(box);
      if ((overflowY === "auto" || overflowY === "scroll") && box.scrollHeight > box.clientHeight) {
        return box.scrollTop;
      }
    }
    return document.scrollingElement?.scrollTop ?? 0;
  });
}

/** How many pages of the parts *Show more parts* reads after the first. */
export function laterPages(about: About): number {
  return Math.ceil(about.part_count / pageSize.usual) - 1;
}

/**
 * Brings the focus from the list of parts to *Show more parts* with Tab and presses it with Enter, then lets the page
 * draw the page of parts that arrived.
 */
export async function pressMoreWithTheKeyboard(page: Page, list: Locator): Promise<void> {
  const more = morePartsButton(page);
  await list.focus();
  await page.keyboard.press("Tab");
  await expect(more).toBeFocused();
  await pressedForAPage(page, () => page.keyboard.press("Enter"));
  await drawn(page);
}

/** How far the mouse scrolls the parts back up: several rows, so the list measures the rows it draws anew. */
const SCROLL_UP_PX = 800;

/** How far above *Show more parts* the mouse stands to scroll: over the rows of the list. */
const ABOVE_THE_BUTTON_PX = 100;

/** How many times the page is let draw while the list measures the rows it scrolled to. */
const DRAWS_AFTER_SCROLL = 15;

/** Where the box around the parts stood: when the mouse pressed, right after its wheel, and after the page settled. */
export interface ScrolledByMouse {
  readonly pressed: number;
  readonly scrolled: number;
  readonly settled: number;
}

/**
 * Presses *Show more parts* with the mouse, then scrolls back up with the wheel over the rows above it, and lets the
 * page draw while the list measures the rows it draws there.
 */
export async function scrollUpAfterAMousePress(page: Page): Promise<ScrolledByMouse> {
  const more = morePartsButton(page);
  await pressedForAPage(page, () => more.click());
  await drawn(page);
  const pressed = await scrolledAround(more);
  const button = await more.boundingBox();
  expect(button).not.toBeNull();
  await page.mouse.move(
    (button?.x ?? 0) + (button?.width ?? 0) / 2,
    (button?.y ?? 0) - ABOVE_THE_BUTTON_PX,
  );
  await page.mouse.wheel(0, -SCROLL_UP_PX);
  await expect.poll(() => scrolledAround(more)).toBeLessThan(pressed);
  const scrolled = await scrolledAround(more);
  await inOrder(Array.from({ length: DRAWS_AFTER_SCROLL }, () => () => drawn(page)));
  return { pressed, scrolled, settled: await scrolledAround(more) };
}

/** Presses each download button and checks that the browser saves the file under the name the server gives it. */
export async function downloadEveryFile(page: Page, about: About): Promise<void> {
  await inOrder(
    OFFERED.map((kind) => async () => {
      const file = about.files.find((each) => each.kind === kind);
      expect(file, `the build carries the ${kind} file`).toBeDefined();
      const saved = page.waitForEvent("download");
      await page.getByRole("button", { name: t(downloadMessages[kind]), exact: true }).click();
      expect((await saved).suggestedFilename()).toBe(file?.name);
    }),
  );
}
