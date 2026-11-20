import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { downloadMessages, OFFERED } from "../../src/features/about/index.ts";
import { pageSize } from "../../src/shared/paged.ts";
import { type ApiSession } from "./api.ts";
import { signedInAdmin } from "./access.ts";
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
      params: { query: { limit: 100, ...(cursor === undefined ? {} : { cursor }) } },
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
  const more = page.getByRole("button", { name: t("about.parts-more"), exact: true });
  const later = Math.ceil(about.part_count / pageSize.usual) - 1;
  expect(later, "the parts take more than one page").toBeGreaterThan(0);
  await inOrder(
    Array.from({ length: later }, () => async () => {
      const answered = page.waitForResponse(
        (answer) => answer.url().includes(PARTS_ADDRESS) && answer.request().method() === "GET",
      );
      await more.click();
      await answered;
    }),
  );
  await expect(more).toHaveCount(0);
  await list.focus();
  await page.keyboard.press("End");
  await expect(list.getByText(last, { exact: true })).toBeVisible();
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
