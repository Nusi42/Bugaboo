import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { formatCalendarDay } from "../../src/shared/format.ts";
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

/** The words About uses for the support period of a release: no last day is a preview. */
export function supportWords(period: About["support_period"]): string {
  const until = period.fixes_until;
  return until === null || until === undefined
    ? t("about.support-preview")
    : t("about.support-until", { date: formatCalendarDay(until) });
}

/** The three downloads, with the button each has. */
const DOWNLOADS = [
  ["cyclonedx", "about.download-cyclonedx"],
  ["spdx", "about.download-spdx"],
  ["notices", "about.download-notices"],
] as const;

/** Presses each download button and checks that the browser saves the file under the name the server gives it. */
export async function downloadEveryFile(page: Page, about: About): Promise<void> {
  await inOrder(
    DOWNLOADS.map(([kind, message]) => async () => {
      const file = about.files.find((each) => each.kind === kind);
      expect(file, `the build carries the ${kind} file`).toBeDefined();
      const saved = page.waitForEvent("download");
      await page.getByRole("button", { name: t(message), exact: true }).click();
      expect((await saved).suggestedFilename()).toBe(file?.name);
    }),
  );
}
