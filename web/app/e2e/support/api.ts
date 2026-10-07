import { CSRF_COOKIE, CSRF_HEADER } from "@bugaboo/api-client";
import { type APIRequestContext, type APIResponse, request } from "@playwright/test";

/** An answer of the server, as the tests read it. */
export type Json = Record<string, unknown>;

/**
 * A person at the other end of the API, for getting the tests to a state quickly: it keeps the
 * cookies of its sign-in and sends the token with every change, like the app does. What the tests
 * are about is still done through the screens.
 */
export interface ApiSession {
  /** Asks for something and returns the answer. */
  readonly get: (path: string) => Promise<Json>;
  /** Changes something and returns the answer (an empty object when there is none). */
  readonly post: (path: string, data?: unknown) => Promise<Json>;
  /** Takes something away. */
  readonly remove: (path: string) => Promise<Json>;
  /** Puts a new value in place of the old one. */
  readonly put: (path: string, data: unknown) => Promise<Json>;
  /** The cookies of the session, to open a browser that is signed in as this person. */
  readonly state: () => Promise<StorageState>;
  /** Signs out (so the session is not left in the list of places) and closes the session. */
  readonly dispose: () => Promise<void>;
}

/** What a browser needs to be signed in as somebody: their cookies. */
export type StorageState = Awaited<ReturnType<APIRequestContext["storageState"]>>;

function isJson(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** The body of an answer that is a success, or an error that says what was asked and answered. */
async function bodyOf(what: string, answer: APIResponse): Promise<Json> {
  if (!answer.ok()) {
    throw new Error(`${what}: ${String(answer.status())} ${await answer.text()}`);
  }
  if (answer.status() === 204) {
    return {};
  }
  const body: unknown = await answer.json();
  if (!isJson(body)) {
    throw new Error(`${what}: the answer is not an object`);
  }
  return body;
}

/** The token of the session, in the header the server wants it in. */
async function tokenOf(context: APIRequestContext): Promise<Record<string, string>> {
  const { cookies } = await context.storageState();
  const csrf = cookies.find(({ name }: { readonly name: string }) => name === CSRF_COOKIE)?.value;
  return csrf === undefined ? {} : { [CSRF_HEADER]: csrf };
}

/** A session of nobody yet, for the server at `origin`. */
export async function openSession(origin: string): Promise<ApiSession> {
  const context = await request.newContext({
    baseURL: origin,
    ignoreHTTPSErrors: true,
    // The server only takes changes that come from its own pages.
    extraHTTPHeaders: { origin },
  });
  return {
    get: async (path) => bodyOf(`GET ${path}`, await context.get(path)),
    post: async (path, data) =>
      bodyOf(
        `POST ${path}`,
        await context.post(path, {
          headers: await tokenOf(context),
          ...(data === undefined ? {} : { data }),
        }),
      ),
    remove: async (path) =>
      bodyOf(`DELETE ${path}`, await context.delete(path, { headers: await tokenOf(context) })),
    put: async (path, data) =>
      bodyOf(`PUT ${path}`, await context.put(path, { headers: await tokenOf(context), data })),
    state: () => context.storageState(),
    dispose: async () => {
      // Nobody may be signed in any more, or the session would be listed forever.
      await context.post("/api/v1/auth/sign-out", { headers: await tokenOf(context) });
      await context.dispose();
    },
  };
}

/** A script with an access key: it sends the key and nothing else, no cookie and no token. */
export interface KeyClient {
  /** Asks for something with the key and returns the status and the code of an error, if any. */
  readonly get: (path: string) => Promise<KeyAnswer>;
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
      const code = isJson(body) && typeof body["code"] === "string" ? body["code"] : undefined;
      return { status: answer.status(), code };
    },
    dispose: () => context.dispose(),
  };
}
