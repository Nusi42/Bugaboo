import { type Page, type Request } from "@playwright/test";

/**
 * Counts the requests a page sends with this method, from now on; with `path`, only the ones to
 * an address that starts with it (`/api/v1/search/issues`).
 */
export function countRequests(page: Page, method: string, path = ""): () => number {
  let sent = 0;
  page.on("request", (request: Readonly<Pick<Request, "method" | "url">>) => {
    if (request.method() === method && new URL(request.url()).pathname.startsWith(path)) {
      sent += 1;
    }
  });
  return () => sent;
}
