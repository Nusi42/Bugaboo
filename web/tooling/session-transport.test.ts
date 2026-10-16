import { once } from "node:events";
import { createServer, type Server } from "node:http";

import { afterEach, describe, expect, it } from "vitest";

import {
  closePool,
  CookieJar,
  exchange,
  type Answered,
} from "../app/e2e/support/session-transport.ts";

const running: Server[] = [];
const origins: string[] = [];

afterEach(() => {
  for (const origin of origins.splice(0)) {
    closePool(origin);
  }
  for (const server of running.splice(0)) {
    server.close();
    server.closeAllConnections();
  }
});

/** A server that counts the connections it gets and says OK, with a cookie. */
async function counting(): Promise<{ origin: string; connections: () => number }> {
  let connections = 0;
  const server = createServer((_request, response) => {
    response.setHeader("set-cookie", ["a=1; Path=/; Secure; HttpOnly", "b=2; Path=/x"]);
    response.end("ok");
  });
  server.on("connection", () => {
    connections += 1;
  });
  running.push(server);
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = server.address();
  if (address === null || typeof address === "string") {
    throw new Error("The server has no port.");
  }
  const origin = `http://127.0.0.1:${String(address.port)}`;
  origins.push(origin);
  return { origin, connections: () => connections };
}

/** `count` requests, one after the other. */
async function askMany(origin: string, count: number): Promise<readonly Answered[]> {
  if (count === 0) {
    return [];
  }
  const first = await exchange({ url: `${origin}/`, method: "GET", headers: {}, body: "" }, origin);
  return [first, ...(await askMany(origin, count - 1))];
}

describe("test_04 the sessions of the tests share their connections (M4 WP-03)", () => {
  it("answers twenty requests one after the other over one connection", async () => {
    const server = await counting();
    const answers = await askMany(server.origin, 20);
    expect(answers.map((answer) => answer.status)).toEqual(Array.from({ length: 20 }, () => 200));
    expect(answers.map((answer) => answer.body)).toEqual(Array.from({ length: 20 }, () => "ok"));
    expect(server.connections()).toBe(1);
  });

  it("opens a new connection after the pool of a stopped server was closed", async () => {
    const server = await counting();
    await askMany(server.origin, 2);
    closePool(server.origin);
    await askMany(server.origin, 2);
    expect(server.connections()).toBe(2);
  });
});

describe("test_05 the cookies of a session are its own (M4 WP-03)", () => {
  it("keeps the cookies of the answers and sends them to the paths they are for", () => {
    const jar = new CookieJar("localhost");
    jar.store(["a=1; Path=/; Secure; HttpOnly; SameSite=Strict", "b=2; Path=/x"]);
    expect(jar.header("/")).toBe("a=1");
    expect(jar.header("/x/y")).toBe("a=1; b=2");
    expect(jar.valueOf("b")).toBe("2");
    expect(jar.all()[0]).toMatchObject({
      domain: "localhost",
      httpOnly: true,
      secure: true,
      sameSite: "Strict",
      expires: -1,
    });
  });

  it("forgets a cookie that the server ended, and keeps two jars apart", () => {
    const first = new CookieJar("localhost");
    const second = new CookieJar("localhost");
    first.store(["a=1; Path=/"]);
    second.store(["a=9; Path=/"]);
    first.store(["a=; Path=/; Max-Age=0"]);
    expect(first.header("/")).toBeUndefined();
    expect(second.header("/")).toBe("a=9");
  });
});
