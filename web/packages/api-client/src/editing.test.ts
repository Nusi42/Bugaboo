import { describe, expect, it } from "vitest";

import { createApi } from "./client.ts";
import { heartbeatSeconds, leaseSeconds, versionsMax } from "./editing.generated.ts";
import {
  inGroupsOfOneCheck,
  readDraft,
  sendHeartbeat,
  startHeartbeatsOn,
  versionsOf,
  type EditWhat,
  type RecordRef,
} from "./editing.ts";

const what: EditWhat = { record: { kind: "issue", id: "i-1" }, part: "description" };

/** What the server was asked. */
interface Sent {
  readonly method: string;
  readonly url: string;
  readonly body: unknown;
}

/** A client whose server answers every request with `answer(body)`, and the requests it got. */
function serving(answer: (body: unknown) => unknown) {
  const sent: Sent[] = [];
  const api = createApi({
    baseUrl: "http://bugaboo.test",
    csrfToken: () => "token",
    fetch: async (request) => {
      const text = await request.clone().text();
      const body: unknown = text === "" ? null : JSON.parse(text);
      sent.push({ method: request.method, url: request.url, body });
      return new Response(JSON.stringify(answer(body)), {
        headers: { "content-type": "application/json" },
      });
    },
  });
  return { client: api.client, sent };
}

describe("iss_17 the numbers of editing come from the server code", () => {
  it("make a heartbeat four times in a lease", () => {
    expect(leaseSeconds).toBe(4 * heartbeatSeconds);
    expect(versionsMax).toBeGreaterThan(1);
  });
});

describe("iss_17 the heartbeat", () => {
  it("sends the lock token with the text so far, and nothing about the draft without one", async () => {
    const beat = { status: "held", expires_at: "x" };
    const { client, sent } = serving(() => ({ lock: beat, notices: [], draft_saved: true }));
    await sendHeartbeat(client, what, "lock-1", { text: "So far", base_version: 3 });
    await sendHeartbeat(client, what, "lock-1");
    expect(sent[0]?.body).toEqual({
      what,
      lock_token: "lock-1",
      draft: { text: "So far", base_version: 3 },
    });
    expect(sent[1]?.body).toEqual({ what, lock_token: "lock-1" });
  });

  it("runs every fifteen seconds, one beat at a time, until it is stopped", async () => {
    const ticks: (() => void)[] = [];
    const stops: number[] = [];
    const timers = {
      every: (run: () => void, milliseconds: number) => {
        expect(milliseconds).toBe(heartbeatSeconds * 1000);
        ticks.push(run);
        return ticks.length;
      },
      stop: (timer: number) => {
        stops.push(timer);
      },
    };
    const gate = Promise.withResolvers<void>();
    let calls = 0;
    const stop = startHeartbeatsOn(() => {
      calls += 1;
      return gate.promise;
    }, timers);
    ticks[0]?.();
    ticks[0]?.();
    expect(calls).toBe(1);
    gate.resolve();
    await Promise.resolve();
    await Promise.resolve();
    ticks[0]?.();
    expect(calls).toBe(2);
    stop();
    expect(stops).toEqual([1]);
  });
});

describe("iss_11 the draft of a person", () => {
  it("is nothing when the server has none", async () => {
    const { client, sent } = serving(() => ({ draft: null }));
    const found = await readDraft(client, { kind: "issue", id: "i-1", part: "description" });
    expect(found).toBeNull();
    expect(sent[0]?.url).toContain("kind=issue");
  });
});

describe("iss_17 the check of versions", () => {
  it("asks for no more than one check takes at once, and gives the answers in order", async () => {
    const records: RecordRef[] = Array.from({ length: versionsMax * 2 + 3 }, (_, at) => ({
      kind: "issue",
      id: `i-${String(at)}`,
    }));
    let call = 0;
    const { client, sent } = serving(() => {
      const group = records.slice(call * versionsMax, (call + 1) * versionsMax);
      call += 1;
      return { items: group.map((record) => ({ record, version: 1 })) };
    });
    const items = await versionsOf(client, records);
    expect(sent).toHaveLength(3);
    expect(sent[0]?.body).toEqual({ records: records.slice(0, versionsMax) });
    expect(sent[2]?.body).toEqual({ records: records.slice(2 * versionsMax) });
    expect(items.map((item) => item.record)).toEqual(records);
  });

  it("splits a list into groups of the size of one check", () => {
    const records: RecordRef[] = Array.from({ length: versionsMax + 1 }, (_, at) => ({
      kind: "issue",
      id: `i-${String(at)}`,
    }));
    expect(inGroupsOfOneCheck(records).map((group) => group.length)).toEqual([versionsMax, 1]);
    expect(inGroupsOfOneCheck([])).toEqual([]);
  });
});
