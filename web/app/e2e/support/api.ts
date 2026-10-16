import {
  ApiError,
  CSRF_COOKIE,
  createApi,
  isProblem,
  type Api,
  type paths,
} from "@bugaboo/api-client";
import { type APIRequestContext } from "@playwright/test";

import { type Answered, CookieJar, exchange } from "./session-transport.ts";

/**
 * A person at the other end of the API, for getting the tests to a state quickly: it keeps the
 * cookies of its sign-in and talks through the client of the app, so the paths and the bodies are
 * typed by the API description, the token is sent the way the app sends it and a refusal is an
 * `ApiError`. What the tests are about is still done through the screens.
 */
export interface ApiSession {
  /** The client of the app, over the cookies of this session. */
  readonly client: Readonly<Api["client"]>;
  /** The cookies of the session, to open a browser that is signed in as this person. */
  readonly state: () => Promise<StorageState>;
  /** Signs out (so the session is not left in the list of places) and closes the session. */
  readonly dispose: () => Promise<void>;
}

/** What a browser needs to be signed in as somebody: their cookies. */
export type StorageState = Awaited<ReturnType<APIRequestContext["storageState"]>>;

/** The answer of the server as the client of the app expects it. */
function responseOf(answer: Answered): Response {
  // Every header one by one: a few cookies are a few headers of the same name.
  const headers = new Headers();
  for (const [name, value] of answer.headers) {
    headers.append(name, value);
  }
  const empty = answer.status === 204 || answer.status === 304;
  return new Response(empty ? null : answer.body, { status: answer.status, headers });
}

/** Sends what the client made through the context of a browser, and gives back what the server said. */
async function send(context: APIRequestContext, asked: Request, origin: string): Promise<Response> {
  const body = await asked.text();
  const answer = await context.fetch(asked.url, {
    method: asked.method,
    // The server only takes changes that come from its own pages.
    headers: { origin, ...Object.fromEntries(asked.headers) },
    ...(body === "" ? {} : { data: body }),
    maxRedirects: 0,
  });
  const headers: (readonly [string, string])[] = [];
  for (const header of answer.headersArray()) {
    headers.push([header.name, header.value]);
  }
  return responseOf({ status: answer.status(), headers, body: await answer.text() });
}

/** Sends what the client made through the shared connections of the server, with the cookies of one session. */
async function sendPooled(jar: CookieJar, asked: Request, origin: string): Promise<Response> {
  const cookie = jar.header(new URL(asked.url).pathname);
  const answer = await exchange(
    {
      url: asked.url,
      method: asked.method,
      // The server only takes changes that come from its own pages.
      headers: {
        origin,
        ...(cookie === undefined ? {} : { cookie }),
        ...Object.fromEntries(asked.headers),
      },
      body: await asked.text(),
    },
    origin,
  );
  jar.store(
    answer.headers.filter(([name]) => name.toLowerCase() === "set-cookie").map(([, line]) => line),
  );
  return responseOf(answer);
}

/** Signs out, so the session is not left in the list of places. */
async function signOut(client: Readonly<Api["client"]>): Promise<void> {
  // A session that never signed in is refused, which is as good.
  await client.POST("/api/v1/auth/sign-out").catch((error: unknown) => {
    if (!(error instanceof ApiError)) {
      throw error;
    }
  });
}

/**
 * A session of nobody yet, for the server at `origin`. Its requests go over the connections that
 * every session of the server shares (see `session-transport.ts`), and its cookies are its own.
 */
export function openSession(origin: string): ApiSession {
  const jar = new CookieJar(new URL(origin).hostname);
  const { client } = createApi({
    baseUrl: origin,
    fetch: (asked) => sendPooled(jar, asked, origin),
    csrfToken: () => jar.valueOf(CSRF_COOKIE),
  });
  return {
    client,
    state: () => Promise.resolve({ cookies: jar.all(), origins: [] }),
    dispose: () => signOut(client),
  };
}

/** The token of the context right now, from its cookie jar: a browser can sign in at any time. */
async function tokenOf(context: APIRequestContext): Promise<string | undefined> {
  const { cookies } = await context.storageState();
  for (const cookie of cookies) {
    if (cookie.name === CSRF_COOKIE) {
      return cookie.value;
    }
  }
  return undefined;
}

/**
 * The session of a context that exists already, like the one of a browser that is signed in: the
 * token is read from its cookie jar at every request, so it is the one that browser holds.
 */
export function sessionOver(context: APIRequestContext, origin: string): ApiSession {
  const { client } = createApi({
    baseUrl: origin,
    fetch: (asked) => send(context, asked, origin),
    csrfToken: () => tokenOf(context),
  });
  return {
    client,
    state: () => context.storageState(),
    dispose: async () => {
      // Nobody may be signed in any more, or the session would be listed forever.
      await signOut(client);
      await context.dispose();
    },
  };
}

/** The routes that can be asked for without saying anything about which thing is meant. */
type PlainRoute = {
  [Path in keyof paths]: paths[Path] extends { readonly get: unknown } ? Path : never;
}[keyof paths];

/** A script with an access key: it sends the key and nothing else, no cookie and no token. */
export interface KeyClient {
  /** Asks for something with the key and returns the status and the code of an error, if any. */
  readonly get: (path: PlainRoute) => Promise<KeyAnswer>;
}

/** What the server said to a script. */
export interface KeyAnswer {
  readonly status: number;
  /** The code of the error (`auth.not_signed_in`), when the answer is one. */
  readonly code: string | undefined;
}

/** A script that uses `key` against the server at `origin`, over the connections all sessions share. */
export function useKey(origin: string, key: string): KeyClient {
  return {
    get: async (path) => {
      const answer = await exchange(
        {
          url: new URL(path, origin).href,
          method: "GET",
          headers: { authorization: `Bearer ${key}` },
          body: "",
        },
        origin,
      );
      const ok = answer.status >= 200 && answer.status < 300;
      const body: unknown = ok ? undefined : JSON.parse(answer.body);
      const problem = isProblem(body) ? body : undefined;
      return { status: answer.status, code: problem?.code };
    },
  };
}
