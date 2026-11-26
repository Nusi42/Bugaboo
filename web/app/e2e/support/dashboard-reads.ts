import { expect, type Page } from "@playwright/test";

import { partIn } from "./parts.ts";

/** Waits until every box of the dashboard on screen shows the answer of its first read: no grey shapes stand in. */
export async function waitForBoxAnswers(page: Page): Promise<void> {
  await expect(partIn(page, "skeleton.line")).toHaveCount(0);
}

/** The reads of what boxes answer that the page sends from now on, until `stop` is called. */
export function boxReadsFrom(page: Page): { readonly stop: () => readonly string[] } {
  const reads: string[] = [];
  const listen = (request: { readonly url: () => string; readonly method: () => string }): void => {
    if (
      request.method() === "GET" &&
      /\/api\/v1\/dashboards\/[^/]+\/boxes\//u.test(request.url())
    ) {
      reads.push(request.url());
    }
  };
  page.on("request", listen);
  return {
    stop: () => {
      page.off("request", listen);
      return reads;
    },
  };
}
