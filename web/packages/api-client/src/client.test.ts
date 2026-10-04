import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import { ApiError, createApi, CSRF_HEADER, isProblem, readCookie } from "./index.ts";

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
