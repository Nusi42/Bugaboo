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

/**
 * Holds every request that matches `url` until the returned function is called, as a slow read
 * would: what the page shows while it waits can be looked at, and the answer comes on cue.
 */
export async function holdUntilReleased(page: Page, url: string): Promise<() => void> {
  const { promise, resolve } = Promise.withResolvers<void>();
  await page.route(url, async (route) => {
    await promise;
    await route.continue();
  });
  return resolve;
}
