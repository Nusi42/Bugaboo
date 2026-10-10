import createFetchClient, { type Middleware } from "openapi-fetch";
import createQueryClient from "openapi-react-query";

import { ApiError, isProblem, PROBLEM_CONTENT_TYPE } from "./problem.ts";
import { CSRF_COOKIE, CSRF_HEADER } from "./protocol.ts";
import { markSafeHtml } from "./safe-html.ts";
import { safeHtmlProperties } from "./safe-html.generated.ts";
import type { paths } from "./schema.ts";

/** The methods that only read; every other method carries the CSRF token. */
const SAFE_METHODS: ReadonlySet<string> = new Set(["GET", "HEAD"]);

/** What the client needs from its surroundings. Tests replace these. */
export interface ClientOptions {
  /** The address of the server; empty (the default) means the page's own address. */
  readonly baseUrl?: string;
  /** Sends a request; the browser's `fetch` by default. */
  readonly fetch?: (request: Request) => Promise<Response>;
  /** The current CSRF token, or `undefined` when there is none (not signed in). */
  readonly csrfToken?: () => string | undefined | Promise<string | undefined>;
}

/** Reads a cookie from a cookie header text, for the CSRF token. */
export function readCookie(name: string, cookies: string): string | undefined {
  for (const pair of cookies.split(";")) {
    const [key, ...rest] = pair.trim().split("=");
    if (key === name) {
      return rest.join("=");
    }
  }
  return undefined;
}

/** The CSRF token of the page: the cookie the server set for it. */
function tokenFromPage(): string | undefined {
  return readCookie(CSRF_COOKIE, document.cookie);
}

/** Turns an answer that is not a success into an {@link ApiError}, with the problem if there is one. */
async function errorOf(response: Response): Promise<ApiError> {
  if (response.headers.get("content-type")?.includes(PROBLEM_CONTENT_TYPE) === true) {
    const body: unknown = await response.json().catch(() => null);
    if (isProblem(body)) {
      return new ApiError(response.status, body);
    }
  }
  return new ApiError(response.status, undefined);
}

/** Whether an answer is JSON the client may read (not a problem, which has its own type). */
function isJson(response: Response): boolean {
  return response.headers.get("content-type")?.includes("application/json") === true;
}

/**
 * Turns the value of every property the server marks as rendered HTML (`x-safe-html`, listed in
 * `safeHtmlProperties`) into a `SafeHtml`. This is the one place in the client that makes one from
 * an answer (M3-F12, SEC-RICHTEXT); the generated types say `SafeHtml` for these properties.
 */
export function readAnswer(text: string): unknown {
  return JSON.parse(text, (key: string, value: unknown) =>
    safeHtmlProperties.has(key) && typeof value === "string" ? markSafeHtml(value) : value,
  );
}

/**
 * The same answer, read with {@link readAnswer} when something asks for its JSON. The length is
 * given so the client reads the body once as text.
 */
async function withSafeHtml(response: Response): Promise<Response> {
  const text = await response.text();
  const headers = new Headers(response.headers);
  headers.set("content-length", String(new TextEncoder().encode(text).length));
  const read = new Response(text, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
  Object.defineProperty(read, "json", { value: () => Promise.resolve(readAnswer(text)) });
  return read;
}

/** Adds the CSRF token to requests that change something, and raises {@link ApiError}s. */
function rules(csrfToken: NonNullable<ClientOptions["csrfToken"]>): Middleware {
  return {
    async onRequest({ request }) {
      if (!SAFE_METHODS.has(request.method)) {
        const token = await csrfToken();
        if (token !== undefined) {
          request.headers.set(CSRF_HEADER, token);
        }
      }
      return request;
    },
    async onResponse({ response }) {
      if (!response.ok) {
        throw await errorOf(response);
      }
      return isJson(response) && response.status !== 204 ? withSafeHtml(response) : response;
    },
    onError() {
      // No answer at all: the network or the server is not there.
      return new ApiError(0, undefined);
    },
  };
}

/**
 * The only way the browser app talks to the server (ARCH-WEB-API). Calls are typed by the
 * OpenAPI document. A request that does not succeed rejects with an {@link ApiError} that holds
 * the problem and the key of the message to show.
 */
export function createApi<Paths extends object = paths>(options: Readonly<ClientOptions> = {}) {
  const csrfToken = options.csrfToken ?? tokenFromPage;
  const client = createFetchClient<Paths>({
    baseUrl: options.baseUrl ?? "",
    credentials: "same-origin",
    ...(options.fetch === undefined ? {} : { fetch: options.fetch }),
  });
  client.use(rules(csrfToken));
  return { client, query: createQueryClient(client) };
}

export type Api = ReturnType<typeof createApi<paths>>;
