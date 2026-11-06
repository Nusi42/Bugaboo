import { type Page } from "@playwright/test";

/**
 * Does the steps one after the other. A browser does one thing at a time, so the steps of a test
 * cannot run side by side.
 */
export async function inOrder(steps: readonly (() => Promise<unknown>)[]): Promise<void> {
  const [first, ...rest] = steps;
  if (first !== undefined) {
    await first();
    await inOrder(rest);
  }
}

/** A step that has nothing to do (a layout where there is nothing to look at, a preparation that needs none). */
export const nothing = (): Promise<void> => Promise.resolve();

/**
 * Counts the changes (`PATCH`, `POST`, `DELETE`) of the person's own data that the server answered
 * on the page, from now on. A test that typed several times waits until it has seen them all
 * before it reloads, or the reload cuts the saves that wait in line.
 */
export function changesSeen(page: Page): () => number {
  let seen = 0;
  page.on("response", (response) => {
    if (response.url().includes("/api/v1/me/") && response.request().method() !== "GET") {
      seen += 1;
    }
  });
  return () => seen;
}

/** The answer for a change made on an old version. */
const CONFLICT = 409;

/** Waits for the answer to a change of a length that says it was changed meanwhile (409). */
export function waitForRefusedChange(page: Page): Promise<unknown> {
  return page.waitForResponse(
    (response) => response.request().method() === "PATCH" && response.status() === CONFLICT,
  );
}
