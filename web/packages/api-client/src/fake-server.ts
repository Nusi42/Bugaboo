/**
 * What a test needs to stand in for the server: a `fetch` for {@link createApi} that hands each
 * request to `respond` as its method, its address and the text of its body. The browser's
 * `Request` cannot be made readonly, so the tests of the app keep it out of their own code and
 * use this (only the API client handles requests; `.oxlintrc.json`).
 */
export function fakeServer(
  respond: (method: string, address: string, body: string) => Response,
): (request: Request) => Promise<Response> {
  return async (request) => respond(request.method, request.url, await request.text());
}
