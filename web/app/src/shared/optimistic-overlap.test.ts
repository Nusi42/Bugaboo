import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";

import { optimisticSend } from "./optimistic.ts";
import { addName, key, nextTick, slowServer, type Names } from "./optimistic-testing.ts";

function twoNames() {
  const client = new QueryClient();
  client.setQueryData<Names>(key, { names: ["Ana"] });
  const servers = new Map<string, ReturnType<typeof slowServer>>();
  const run = optimisticSend(
    () => client,
    addName(({ name }) => {
      const server = slowServer();
      servers.set(name, server);
      return server.promise;
    }),
  );
  return { client, servers, run };
}

describe("ux_06 changes that overlap", () => {
  it("roll back only the one the server refused, and keep the other", async () => {
    const { client, servers, run } = twoNames();
    const ben = run({ name: "Ben" });
    const cy = run({ name: "Cy" });
    await nextTick();
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Ben", "Cy"]);

    servers.get("Ben")?.refuse(new Error("refused"));
    await expect(ben).rejects.toThrow("refused");
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Cy"]);

    servers.get("Cy")?.answer("ok");
    await expect(cy).resolves.toBe("ok");
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Cy"]);
  });

  it("keep the first one when the second is refused, whatever order the answers come in", async () => {
    const { client, servers, run } = twoNames();
    const ben = run({ name: "Ben" });
    const cy = run({ name: "Cy" });
    await nextTick();
    servers.get("Cy")?.refuse(new Error("refused"));
    await expect(cy).rejects.toThrow("refused");
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Ben"]);
    servers.get("Ben")?.answer("ok");
    await expect(ben).resolves.toBe("ok");
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Ben"]);
  });
});

describe("ux_06 reading the data again after overlapping changes", () => {
  it("read the data again once, when the last change that is on its way is done", async () => {
    const { client, servers, run } = twoNames();
    const refetch = vi.spyOn(client, "invalidateQueries");
    const ben = run({ name: "Ben" });
    const cy = run({ name: "Cy" });
    await nextTick();
    servers.get("Ben")?.answer("ok");
    await ben;
    expect(refetch).not.toHaveBeenCalled();
    servers.get("Cy")?.answer("ok");
    await cy;
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it("do not wait for the refetch: Saved comes when the server has answered", async () => {
    const client = new QueryClient();
    client.setQueryData<Names>(key, { names: ["Ana"] });
    vi.spyOn(client, "invalidateQueries").mockReturnValue(Promise.withResolvers<void>().promise);
    const run = optimisticSend(
      () => client,
      addName(() => Promise.resolve("ok")),
    );
    await expect(run({ name: "Ben" })).resolves.toBe("ok");
  });
});
