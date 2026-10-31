import { mkdirSync } from "node:fs";

import { type Browser, expect, type Page } from "@playwright/test";

import { signIn } from "./flows.ts";
import { Recording } from "./recording.ts";
import { type Instance } from "./server.ts";
import { personOf } from "./showcase-sample.ts";
import { openDevice } from "./showcase.ts";

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

/** An issue of the sample that the pictures show, and the one the recordings open. */
export const ISSUE = "WEB-12";

/** Waits for a page to have its first rows of issues. */
export async function expectRows(page: Page): Promise<void> {
  await expect(page.locator("[data-part='list-row']").first()).toBeVisible();
}

/** Waits for a page to have its heading and for what it asks of the server to have come. */
export async function expectHeading(page: Page): Promise<void> {
  await expect(page.getByRole("heading", { level: 1 }).first()).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** The window of an administrator of the sample, on a computer or on a phone, signed in. */
export async function administratorPage(
  browser: Browser,
  server: Instance,
  device: "computer" | "phone",
): Promise<Page> {
  const context = await openDevice(browser, server, device);
  const page = await context.newPage();
  await signIn(page, personOf.administrator());
  return page;
}

/** A recording of the window of an administrator of the sample on a computer. */
export async function recordingOf(
  browser: Browser,
  server: Instance,
): Promise<{ readonly page: Page; readonly recording: Recording }> {
  const page = await administratorPage(browser, server, "computer");
  return { page, recording: new Recording(page) };
}
