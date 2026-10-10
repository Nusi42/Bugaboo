import { type Page } from "@playwright/test";

/** Makes the answer to asking for an edit lock late, as a slow connection would. */
export async function holdBackLocks(page: Page, milliseconds: number): Promise<void> {
  await page.route("**/api/v1/edit-locks", async (route) => {
    if (route.request().method() === "POST") {
      await new Promise<void>((done) => {
        setTimeout(done, milliseconds);
      });
    }
    await route.continue();
  });
}
