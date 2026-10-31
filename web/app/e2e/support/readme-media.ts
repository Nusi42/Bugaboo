import { mkdirSync } from "node:fs";
import { join } from "node:path";

import { type Browser, expect, type Page } from "@playwright/test";

import { expectLook } from "./checks.ts";
import { signIn } from "./flows.ts";
import { accountOf, readmeTheme } from "./readme-organization.ts";
import { Recording } from "./recording.ts";
import { pointerAway } from "./screenshots.ts";
import { type Instance } from "./server.ts";
import { deviceContext, type DeviceOptions } from "./showcase.ts";

/** The folder of the pictures and recordings of the front page, from `BB_README_MEDIA_DIR`. */
export const mediaFolder = process.env["BB_README_MEDIA_DIR"];

/** The folder of the pictures, made if it is missing. */
export function folder(): string {
  if (mediaFolder === undefined) {
    throw new TypeError("Set BB_README_MEDIA_DIR to make the pictures.");
  }
  mkdirSync(mediaFolder, { recursive: true });
  return mediaFolder;
}

/**
 * How many pixels of a picture one pixel of the page is: twice as many, so the text stays sharp on
 * the front page, which shows the window of a computer smaller than it is. The front picture, which
 * shows the whole window in four themes, is taken at one and a half: at twice it would be larger
 * than a file in Git may be (GIT-FILES).
 */
export const SHARP = { pictures: 2, front: 1.5 } as const;

/** The windows of the pictures: sharp, and with less motion, so no picture is taken while something fades in. */
function windowOf(scale: number): DeviceOptions {
  return { deviceScaleFactor: scale, reducedMotion: "reduce" };
}

/**
 * How far the clock of the browser is ahead of the moment the workshop was set up: what the
 * workshop did then reads "yesterday" in the pictures, not "a few seconds ago".
 */
const A_DAY_LATER_MS = 24 * 60 * 60 * 1000;

/** Waits for a page to have its first rows of issues. */
export async function expectRows(page: Page): Promise<void> {
  await expect(page.locator("[data-part='list-row']").first()).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** Waits for a page to have its heading and for what it asks of the server to have come. */
export async function expectHeading(page: Page): Promise<void> {
  await expect(page.getByRole("heading", { level: 1 }).first()).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** The window of Marta, the administrator of the workshop, on a computer or on a phone, signed in. */
export async function administratorPage(
  browser: Browser,
  server: Instance,
  device: "computer" | "phone",
  scale: number = SHARP.pictures,
): Promise<Page> {
  const context = await deviceContext(browser, server, device, windowOf(scale));
  await context.clock.setFixedTime(Date.now() + A_DAY_LATER_MS);
  const page = await context.newPage();
  await signIn(page, accountOf("marta"));
  return page;
}

/** Takes what the window shows, in the theme of the pictures (`readmeTheme`), as `<name>.png`. */
export async function shootPicture(page: Page, name: string): Promise<void> {
  await pointerAway(page);
  await expectLook(page, readmeTheme);
  await page.screenshot({ path: join(folder(), `${name}.png`) });
}

/** A recording of the window of Marta on a computer. */
export async function recordingOf(
  browser: Browser,
  server: Instance,
): Promise<{ readonly page: Page; readonly recording: Recording }> {
  const page = await administratorPage(browser, server, "computer");
  return { page, recording: new Recording(page) };
}
