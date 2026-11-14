import { fakeServer } from "@bugaboo/api-client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { sendChange } from "./issue-requests.ts";
import { keepTimeFactor, timeFactorQuery, type TimeFactor } from "./time-factor.ts";

/**
 * What the server of these tests answers: a change with `answer`; a read of the time factor with
 * `factor`, once `factorRead` lets it go.
 */
const server = vi.hoisted(() => ({
  answer: {} as Record<string, unknown>,
  factor: {} as Record<string, unknown>,
  factorRead: Promise.resolve(),
}));

vi.mock("../../shared/api.ts", async () => {
  const { createApi } = await import("@bugaboo/api-client");
  return {
    api: createApi({
      baseUrl: "http://bugaboo.test",
      csrfToken: () => "token",
      fetch: fakeServer(async (method, address) => {
        if (method === "GET" && address.endsWith("/time-factor")) {
          await server.factorRead;
          return Response.json(server.factor);
        }
        return Response.json(server.answer);
      }),
    }),
  };
});

const address = "WEB-142";
const factorKey = queryKeys.issues.time("factor", address);
const projectTime = queryKeys.issues.time("project", "p1");

/** The time factor of the issue at `version`, set to `percent`. */
function factorAt(version: number, percent: number): TimeFactor {
  return {
    issue: "i1",
    version,
    percent,
    set_by: null,
    own_estimate: false,
    own_percent: null,
    person: null,
    may_change: true,
  };
}

afterEach(() => {
  queryClient.clear();
});

describe("plan_14 the time factor shows with a change of the people", () => {
  it("writes the factor the answer tells and does not ask for it again", async () => {
    queryClient.setQueryData(factorKey, factorAt(4, 150));
    queryClient.setQueryData(projectTime, { total: 0 });
    server.answer = { id: "i1", readable_id: address, version: 5, time_factor: factorAt(5, 100) };
    await sendChange("i1", { version: 4, values: { assignedTo: [] } });
    expect(queryClient.getQueryData(factorKey)).toEqual(factorAt(5, 100));
    expect(queryClient.getQueryState(factorKey)?.isInvalidated).toBe(false);
    // The other times of the issue are read again: a change counts in the totals.
    expect(queryClient.getQueryState(projectTime)?.isInvalidated).toBe(true);
  });

  it("reads the factor again after a change that does not tell it", async () => {
    queryClient.setQueryData(factorKey, factorAt(4, 150));
    server.answer = { id: "i1", readable_id: address, version: 5 };
    await sendChange("i1", { version: 4, values: { title: "Login fails" } });
    expect(queryClient.getQueryState(factorKey)?.isInvalidated).toBe(true);
  });

  it("never takes the factor back to an older version", () => {
    queryClient.setQueryData(factorKey, factorAt(6, 200));
    keepTimeFactor(address, factorAt(5, 100));
    expect(queryClient.getQueryData(factorKey)).toEqual(factorAt(6, 200));
    keepTimeFactor(address, factorAt(7, 120));
    expect(queryClient.getQueryData(factorKey)).toEqual(factorAt(7, 120));
  });

  it("keeps the answered factor when a read started before the change ends after it", async () => {
    queryClient.setQueryData(factorKey, factorAt(4, 150));
    const gate = Promise.withResolvers<void>();
    server.factorRead = gate.promise;
    server.factor = factorAt(5, 150);
    const read = queryClient.query({ ...timeFactorQuery(address), staleTime: 0 }).catch(() => null);
    keepTimeFactor(address, factorAt(6, 100));
    gate.resolve();
    await read;
    expect(queryClient.getQueryData(factorKey)).toEqual(factorAt(6, 100));
  });

  it("never lets a read take the factor back to an older version", async () => {
    queryClient.setQueryData(factorKey, factorAt(6, 100));
    server.factorRead = Promise.resolve();
    server.factor = factorAt(5, 150);
    await queryClient.query({ ...timeFactorQuery(address), staleTime: 0 });
    expect(queryClient.getQueryData(factorKey)).toEqual(factorAt(6, 100));
  });
});
