import { once } from "node:events";
import { createServer, type Server } from "node:http";
import { setTimeout as sleep } from "node:timers/promises";

import { afterEach, describe, expect, it } from "vitest";

import { askLive, waitUntilReady, type Ask } from "../app/e2e/support/ready.ts";

const running: Server[] = [];

afterEach(() => {
  for (const server of running.splice(0)) {
    server.close();
    server.closeAllConnections();
  }
});

/** A port nobody listens on right now. */
async function freePort(): Promise<number> {
  const probe = createServer();
  probe.listen(0, "127.0.0.1");
  await once(probe, "listening");
  const address = probe.address();
  probe.close();
  await once(probe, "close");
  if (address === null || typeof address === "string") {
    throw new Error("The probe has no port.");
  }
  return address.port;
}

/** A server that starts to answer `late` milliseconds from now; counts the connections it gets. */
async function answersLate(late: number): Promise<{ port: number; connections: () => number }> {
  const port = await freePort();
  let connections = 0;
  const server = createServer((_request, response) => {
    response.statusCode = 200;
    response.end("ok");
  });
  server.on("connection", () => {
    connections += 1;
  });
  running.push(server);
  void sleep(late).then(() => server.listen(port, "127.0.0.1"));
  return { port, connections: () => connections };
}

/** The pauses between the times, in milliseconds. */
function gapsOf(times: readonly number[]): number[] {
  return times.slice(1).map((time, at) => time - (times[at] ?? time));
}

/** The question `ask`, counted. */
function counted(ask: Ask): { ask: Ask; asked: () => number } {
  let asked = 0;
  return {
    ask: () => {
      asked += 1;
      return ask();
    },
    asked: () => asked,
  };
}

const alive = { exited: () => false, output: () => "" };

describe("test_01 the wait for a server does not storm it with connections (M4 WP-03)", () => {
  it("asks only a handful of times before a server that answers late, and connects once", async () => {
    const late = await answersLate(900);
    const question = counted(askLive(late.port, false));
    await waitUntilReady(question.ask, alive);
    // 50, 100, 200, 400 ms of pauses: about six questions in 900 ms, not nine (every 100 ms) or
    // eighteen (every 50 ms).
    expect(question.asked()).toBeLessThanOrEqual(7);
    expect(late.connections()).toBe(1);
  });

  it("makes longer pauses between questions up to a limit", async () => {
    const times: number[] = [];
    const never: Ask = () => {
      times.push(Date.now());
      return Promise.resolve(false);
    };
    await expect(waitUntilReady(never, alive, 3500)).rejects.toThrow(/did not answer/u);
    expect(Math.max(...gapsOf(times))).toBeGreaterThanOrEqual(900);
    expect(times.length).toBeLessThanOrEqual(10);
  });

  it("gives up with what the program said when it ends while starting", async () => {
    const ended = { exited: () => true, output: () => "port busy" };
    await expect(waitUntilReady(() => Promise.resolve(false), ended)).rejects.toThrow(/port busy/u);
  });
});
