import { type Page, type Request } from "@playwright/test";

/** Counts the requests a page sends with this method, from now on. */
export function countRequests(page: Page, method: string): () => number {
  let sent = 0;
  page.on("request", (request: Readonly<Pick<Request, "method">>) => {
    if (request.method() === method) {
      sent += 1;
    }
  });
  return () => sent;
}
