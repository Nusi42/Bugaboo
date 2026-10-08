import {
  ApiError,
  CSRF_COOKIE,
  createApi,
  isProblem,
  readCookie,
  type Api,
  type paths,
} from "@bugaboo/api-client";
import { type APIRequestContext, request } from "@playwright/test";

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

/** Sends what the client made through the session, and gives back what the server said. */
async function send(
  context: APIRequestContext,
  asked: Request,
  origin: string,
  onCookie: (setCookie: string) => void,
): Promise<Response> {
  const body = await asked.text();
  const answer = await context.fetch(asked.url, {
    method: asked.method,
    // The server only takes changes that come from its own pages.
    headers: { origin, ...Object.fromEntries(asked.headers) },
    ...(body === "" ? {} : { data: body }),
    maxRedirects: 0,
  });
  // Every header one by one: a few cookies are a few headers of the same name.
  const headers = new Headers();
  for (const header of answer.headersArray()) {
    headers.append(header.name, header.value);
    if (header.name.toLowerCase() === "set-cookie") {
      onCookie(header.value);
    }
  }
  const empty = answer.status() === 204 || answer.status() === 304;
  return new Response(empty ? null : await answer.text(), { status: answer.status(), headers });
}

/** A session of nobody yet, for the server at `origin`. */
export async function openSession(origin: string): Promise<ApiSession> {
  return sessionOver(
    await request.newContext({
      baseURL: origin,
      ignoreHTTPSErrors: true,
    }),
    origin,
  );
}

/**
 * The session of a context that exists already, like the one of a browser that is signed in: the
 * token that browser holds is the one the client sends from then on.
 */
export async function sessionOver(context: APIRequestContext, origin: string): Promise<ApiSession> {
  const { cookies } = await context.storageState();
  let token: string | undefined;
  for (const cookie of cookies) {
    if (cookie.name === CSRF_COOKIE) {
      token = cookie.value;
    }
  }
  const { client } = createApi({
    baseUrl: origin,
    fetch: (asked) =>
      send(context, asked, origin, (setCookie) => {
        token = readCookie(CSRF_COOKIE, setCookie) ?? token;
      }),
    csrfToken: () => token,
  });
  return {
    client,
    state: () => context.storageState(),
    dispose: async () => {
      // Nobody may be signed in any more, or the session would be listed forever. A session that
      // never signed in is refused, which is as good.
      await client.POST("/api/v1/auth/sign-out").catch((error: unknown) => {
        if (!(error instanceof ApiError)) {
          throw error;
        }
      });
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
  readonly dispose: () => Promise<void>;
}

/** What the server said to a script. */
export interface KeyAnswer {
  readonly status: number;
  /** The code of the error (`auth.not_signed_in`), when the answer is one. */
  readonly code: string | undefined;
}

/** A script that uses `key` against the server at `origin`. */
export async function useKey(origin: string, key: string): Promise<KeyClient> {
  const context = await request.newContext({
    baseURL: origin,
    ignoreHTTPSErrors: true,
    extraHTTPHeaders: { authorization: `Bearer ${key}` },
  });
  return {
    get: async (path) => {
      const answer = await context.get(path);
      const body: unknown = answer.ok() ? undefined : await answer.json();
      const problem = isProblem(body) ? body : undefined;
      return { status: answer.status(), code: problem?.code };
    },
    dispose: () => context.dispose(),
  };
}
