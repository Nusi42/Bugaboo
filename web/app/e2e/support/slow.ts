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

/** A read that is held: when it was asked for, and the way to let it go. */
export interface Held {
  /** Settles once the page has asked for it. */
  readonly asked: Promise<void>;
  readonly release: () => void;
}

/**
 * Holds every request that matches `url` until `release` is called, as a slow read would: what the
 * page shows while it waits can be looked at, and the answer comes on cue.
 */
export async function holdUntilReleased(page: Page, url: string | Readonly<RegExp>): Promise<Held> {
  const released = Promise.withResolvers<void>();
  const asked = Promise.withResolvers<void>();
  await page.route(url, async (route) => {
    asked.resolve();
    await released.promise;
    await route.continue();
  });
  return { asked: asked.promise, release: released.resolve };
}
