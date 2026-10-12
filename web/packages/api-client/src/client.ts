import createFetchClient, { type Middleware } from "openapi-fetch";
import createQueryClient from "openapi-react-query";

import { ApiError, isProblem, PROBLEM_CONTENT_TYPE } from "./problem.ts";
import { CSRF_COOKIE, CSRF_HEADER } from "./protocol.ts";
import { markSafeHtml } from "./safe-html.ts";
import { safeHtmlPaths } from "./safe-html.generated.ts";
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

/** A place in an answer: the keys from the top down to the string, `*` for every item of a list. */
type Place = readonly string[];

/** Marks the string at `place` below `value`, and every string at the places below a list. */
function markAt(value: unknown, place: Place): unknown {
  const [step, ...rest] = place;
  if (step === undefined) {
    return typeof value === "string" ? markSafeHtml(value) : value;
  }
  if (step === "*" && Array.isArray(value)) {
    return value.map((item: unknown) => markAt(item, rest));
  }
  if (typeof value === "object" && value !== null && !Array.isArray(value)) {
    const copy: Record<string, unknown> = {};
    for (const [key, inner] of Object.entries(value)) {
      copy[key] = key === step ? markAt(inner, rest) : inner;
    }
    return copy;
  }
  return value;
}

/**
 * Reads an answer and turns the strings at the places the server marks as rendered HTML
 * (`x-safe-html`, listed per operation in `safeHtmlPaths`) into a `SafeHtml`. This is the one
 * place in the client that makes one from an answer (M3-F12, SEC-RICHTEXT); the generated types
 * say `SafeHtml` for these properties. A key of the same name anywhere else stays text.
 */
export function readAnswer(text: string, places: readonly Place[]): unknown {
  return places.reduce((answer: unknown, place) => markAt(answer, place), JSON.parse(text));
}

/**
 * The same answer, read with {@link readAnswer} when something asks for its JSON. The length is
 * given so the client reads the body once as text.
 */
async function withSafeHtml(response: Response, places: readonly Place[]): Promise<Response> {
  const text = await response.text();
  const headers = new Headers(response.headers);
  headers.set("content-length", String(new TextEncoder().encode(text).length));
  const read = new Response(text, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
  Object.defineProperty(read, "json", { value: () => Promise.resolve(readAnswer(text, places)) });
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
    async onResponse({ request, response, schemaPath }) {
      if (!response.ok) {
        throw await errorOf(response);
      }
      const places = safeHtmlPaths[`${request.method} ${schemaPath}`];
      return places !== undefined && isJson(response) && response.status !== 204
        ? withSafeHtml(response, places)
        : response;
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
