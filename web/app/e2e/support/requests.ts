import { type Page, type Request, type Response } from "@playwright/test";

/** Whether a request has this method and goes to an address that starts with `path`. */
function sentAs(request: Readonly<Pick<Request, "method" | "url">>, method: string, path: string) {
  return request.method() === method && new URL(request.url()).pathname.startsWith(path);
}

/**
 * Counts the requests a page sends with this method, from now on; with `path`, only the ones to
 * an address that starts with it (`/api/v1/search/issues`).
 */
export function countRequests(page: Page, method: string, path = ""): () => number {
  let sent = 0;
  page.on("request", (request: Readonly<Pick<Request, "method" | "url">>) => {
    if (sentAs(request, method, path)) {
      sent += 1;
    }
  });
  return () => sent;
}

/** The answer to the next request the page sends with this method to an address that starts with `path`. */
export function answerTo(page: Page, method: string, path: string): Promise<Response> {
  return page.waitForResponse((response: Readonly<Response>) =>
    sentAs(response.request(), method, path),
  );
}
