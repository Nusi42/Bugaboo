import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import {
  ApiError,
  createApi,
  conflictOf,
  CSRF_HEADER,
  dataOf,
  isProblem,
  isSafeHtml,
  readCookie,
} from "./index.ts";
import { readAnswer } from "./client.ts";

const BASE = "http://bugaboo.test";

/** A route the real API does not have yet, to see how changes are sent. */
interface ChangePaths {
  "/api/v1/things": {
    get?: never;
    post: {
      parameters: { query?: never; header?: never; path?: never; cookie?: never };
      requestBody: { content: { "application/json": { name: string } } };
      responses: {
        200: { headers: Record<string, unknown>; content: { "application/json": { id: string } } };
      };
    };
  };
}

function ok(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    headers: { "content-type": "application/json" },
  });
}

function problem(code: string, messageKey: string, status: number): Response {
  const body = { type: code, title: "x", status, code, message_key: messageKey, params: {} };
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/problem+json" },
  });
}

/** A client whose server always answers with `answer`. */
function answering(answer: () => Promise<Response>) {
  return createApi({ baseUrl: BASE, fetch: answer });
}

/** What a read of the example route throws when the server answers with `answer`. */
function failureOf(answer: () => Promise<Response>): Promise<unknown> {
  return answering(answer)
    .client.GET("/api/v1/status")
    .catch((error: unknown) => error);
}

/** A client for the route with a body, and the requests it sent. */
function recording(token: string | undefined) {
  const sent: Request[] = [];
  const api = createApi<ChangePaths>({
    baseUrl: BASE,
    csrfToken: () => token,
    fetch: (request) => {
      sent.push(request);
      return Promise.resolve(ok({ id: "a" }));
    },
  });
  return { api, sent };
}

describe("adr_0008 the generated client", () => {
  it("round-trips the example route and types the answer from the OpenAPI document", async () => {
    const seen: string[] = [];
    const api = answering(() => Promise.resolve(ok({ status: "ok" })));
    const spy = createApi({
      baseUrl: BASE,
      fetch: (request) => {
        seen.push(`${request.method} ${request.url}`);
        return Promise.resolve(ok({ status: "ok" }));
      },
    });
    const { data } = await spy.client.GET("/api/v1/status");
    expect(data?.status).toBe("ok");
    expect(seen).toEqual([`GET ${BASE}/api/v1/status`]);
    expect((await api.client.GET("/api/v1/status")).data?.status).toBe("ok");
  });

  it("feeds TanStack Query through the same client", async () => {
    const api = answering(() => Promise.resolve(ok({ status: "ok" })));
    const options = api.query.queryOptions("get", "/api/v1/status");
    expect(options.queryKey).toEqual(["get", "/api/v1/status"]);
    const data = await new QueryClient().query(options);
    expect(data.status).toBe("ok");
  });
});

describe("m1_f29 answers with lists and open objects", () => {
  it("keeps a list a list, so a screen can map over it", async () => {
    const sessions = [
      { id: "s1", started_at: "t", last_seen_at: "t", current: true },
      { id: "s2", started_at: "t", last_seen_at: "t", current: false },
    ];
    const api = answering(() => Promise.resolve(ok({ sessions, truncated: false })));
    const { data } = await api.client.GET("/api/v1/auth/sessions");
    // `map` only compiles when the generated type of the answer is a real list.
    expect(data?.sessions.map((session) => session.id)).toEqual(["s1", "s2"]);
  });

  it("takes a passkey answer of any shape, as the standard writes it", async () => {
    const sent: unknown[] = [];
    const api = createApi({
      baseUrl: BASE,
      csrfToken: () => readCookie("none", ""),
      fetch: async (request) => {
        sent.push(await request.json());
        return ok({ person: null, second_step: null });
      },
    });
    const credential = { id: "abc", response: { clientDataJSON: "x" }, type: "public-key" };
    await api.client.POST("/api/v1/auth/passkey/sign-in", {
      body: { challenge_id: "c", credential },
    });
    expect(sent).toEqual([{ challenge_id: "c", credential }]);
  });
});

describe("ux_07 errors from the server", () => {
  it("turn problem+json into an error with the message key to show", async () => {
    const failure = await failureOf(() =>
      Promise.resolve(problem("rate_limit.exceeded", "error.rate-limit.exceeded", 429)),
    );
    expect(failure).toBeInstanceOf(ApiError);
    expect(failure).toMatchObject({
      status: 429,
      messageKey: "error.rate-limit.exceeded",
      problem: { code: "rate_limit.exceeded" },
    });
  });

  it("carry the seconds the server asks to wait", async () => {
    const answer = problem("rate_limit.exceeded", "error.rate-limit.exceeded", 429);
    answer.headers.set("Retry-After", "7");
    const failure = await failureOf(() => Promise.resolve(answer));
    expect(failure).toMatchObject({ retryAfterSeconds: 7 });
    const none = await failureOf(() =>
      Promise.resolve(problem("rate_limit.exceeded", "error.rate-limit.exceeded", 429)),
    );
    expect(none).toMatchObject({ retryAfterSeconds: undefined });
  });

  it("never show a message key the app does not have", async () => {
    const failure = await failureOf(() =>
      Promise.resolve(problem("server.internal", "error.from.the.future", 500)),
    );
    expect(failure).toMatchObject({
      status: 500,
      messageKey: "error.server.unavailable",
    });
  });

  it("never show text from a proxy", async () => {
    const failure = await failureOf(() =>
      Promise.resolve(new Response("<h1>Bad gateway</h1>", { status: 502 })),
    );
    expect(failure).toMatchObject({ status: 502, messageKey: "error.server.unavailable" });
    expect(failure).not.toHaveProperty("problem.code");
  });

  it("are status 0 when no answer came at all", async () => {
    const failure = await failureOf(() => Promise.reject(new TypeError("offline")));
    expect(failure).toMatchObject({ status: 0 });
  });
});

describe("sec_06 the CSRF header", () => {
  it("goes with changes", async () => {
    const { api, sent } = recording("token-1");
    await api.client.POST("/api/v1/things", { body: { name: "x" } });
    expect(sent[0]?.headers.get(CSRF_HEADER)).toBe("token-1");
  });

  it("is left out when there is no token", async () => {
    const { api, sent } = recording(readCookie("none", ""));
    await api.client.POST("/api/v1/things", { body: { name: "x" } });
    expect(sent[0]?.headers.has(CSRF_HEADER)).toBe(false);
  });

  it("never goes with reads", async () => {
    const sent: Request[] = [];
    const reader = createApi({
      baseUrl: BASE,
      csrfToken: () => "token-1",
      fetch: (request) => {
        sent.push(request);
        return Promise.resolve(ok({ status: "ok" }));
      },
    });
    await reader.client.GET("/api/v1/status");
    expect(sent[0]?.headers.has(CSRF_HEADER)).toBe(false);
  });
});

describe("adr_0008 helpers", () => {
  it("read one cookie from a cookie header", () => {
    expect(readCookie("__Host-bb_csrf", "a=1; __Host-bb_csrf=abc=; b=2")).toBe("abc=");
    expect(readCookie("missing", "a=1")).toBeUndefined();
  });

  it("recognise the problem body and nothing else", () => {
    const body = { code: "x.y", message_key: "k", status: 400, title: "t" };
    expect(isProblem(body)).toBe(true);
    expect(isProblem({ ...body, status: "400" })).toBe(false);
    expect(isProblem(null)).toBe(false);
    expect(isProblem("text")).toBe(false);
  });
});

/** The value below `answer` along these keys (a list is entered by its index). */
function at(answer: unknown, ...keys: readonly (string | number)[]): unknown {
  return keys.reduce(
    (value: unknown, key) =>
      (typeof value === "object" && value !== null
        ? Reflect.get(value, key)
        : undefined) as unknown,
    answer,
  );
}

describe("m3_f12 server-rendered HTML becomes SafeHtml", () => {
  const body = { description_html: "<p>Hi</p>", description: "<p>Hi</p>" };

  it("marks the property of the operation that the description lists and no other string", async () => {
    const api = answering(() => Promise.resolve(ok(body)));
    const answer = await api.client.GET("/api/v1/issues/{issue}", {
      params: { path: { issue: "WEB-1" } },
    });
    const data = new Map(Object.entries(dataOf(answer)));
    expect(isSafeHtml(data.get("description_html"))).toBe(true);
    expect(isSafeHtml(data.get("description"))).toBe(false);
    expect(data.get("description")).toBe("<p>Hi</p>");
  });

  it("leaves a key of the same name in any other answer as text", async () => {
    const api = answering(() => Promise.resolve(ok(body)));
    const data = new Map(Object.entries(dataOf(await api.client.GET("/api/v1/status"))));
    expect(isSafeHtml(data.get("description_html"))).toBe(false);
    expect(data.get("description_html")).toBe("<p>Hi</p>");
  });

  it("marks only along the path: a list, an inner object, and not the same key beside it", () => {
    const text = JSON.stringify({
      items: [{ body: { html: "<p>1</p>" }, html: "plain" }, { body: { html: "<p>2</p>" } }],
      html: "top",
    });
    const read = readAnswer(text, [["items", "*", "body", "html"]]);
    expect(isSafeHtml(at(read, "items", 0, "body", "html"))).toBe(true);
    expect(isSafeHtml(at(read, "items", 1, "body", "html"))).toBe(true);
    expect(at(read, "items", 0, "html")).toBe("plain");
    expect(at(read, "html")).toBe("top");
  });
});

describe("adr_0019 the structured answer to a stale version", () => {
  const stale = {
    type: "edit.version_changed",
    title: "x",
    status: 409,
    code: "edit.version_changed",
    message_key: "error.edit.version-changed",
    params: {},
    conflict: { version: 4, fields: ["priority"], current: { priority: "critical" } },
  } as const;

  it("hands out who changed what, from the error of a change", async () => {
    const error = await failureOf(() =>
      Promise.resolve(
        new Response(JSON.stringify(stale), {
          status: 409,
          headers: { "content-type": "application/problem+json" },
        }),
      ),
    );
    expect(conflictOf(error)?.fields).toEqual(["priority"]);
    expect(conflictOf(error)?.version).toBe(4);
  });

  it("is nothing for any other error and for a conflict that is not one", () => {
    expect(conflictOf(new Error("x"))).toBeUndefined();
    expect(conflictOf(new ApiError(404, undefined))).toBeUndefined();
    const body = { ...stale, conflict: { fields: "no" } };
    const broken = new ApiError(409, body);
    expect(conflictOf(broken)).toBeUndefined();
  });
});
