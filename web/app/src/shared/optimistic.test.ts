import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import { optimisticSend, saveStateOf, summarizeCalls } from "./optimistic.ts";
import { addName, key, nextTick, slowServer, type Names } from "./optimistic-testing.ts";

describe("ux_06 a change shows at once", () => {
  it("is in the cache within 100 ms, while the slow server has not answered", async () => {
    const client = new QueryClient();
    client.setQueryData<Names>(key, { names: ["Ana"] });
    const server = slowServer();
    const run = optimisticSend(
      () => client,
      addName(() => server.promise),
    );

    const started = performance.now();
    const done = run({ name: "Ben" });
    await nextTick();
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Ben"]);
    expect(performance.now() - started).toBeLessThan(100);

    server.answer("ok");
    await expect(done).resolves.toBe("ok");
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Ben"]);
  });

  it("goes back to the old data when the server refuses, and passes the error on", async () => {
    const client = new QueryClient();
    client.setQueryData<Names>(key, { names: ["Ana"] });
    const server = slowServer();
    const run = optimisticSend(
      () => client,
      addName(() => server.promise),
    );

    const done = run({ name: "Ben" });
    await nextTick();
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Ben"]);

    server.refuse(new Error("refused"));
    await expect(done).rejects.toThrow("refused");
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana"]);
  });

  it("says Saving, Saved or Not saved yet for each stage of a change", () => {
    expect(saveStateOf("idle")).toBe("idle");
    expect(saveStateOf("pending")).toBe("saving");
    expect(saveStateOf("success")).toBe("saved");
    expect(saveStateOf("error")).toBe("not-saved");
  });
});

describe("ux_06 what is said when changes overlap", () => {
  const refused = new Error("refused");

  it("keeps an earlier refusal when a later change is accepted: Saved never stands alone", () => {
    const summary = summarizeCalls([
      { status: "error", error: refused },
      { status: "success", error: null },
    ]);
    expect(summary.saveState).toBe("not-saved");
    expect(summary.error).toBe(refused);
  });

  it("says saving while one is still on its way, and shows the refusal once all are done", () => {
    expect(
      summarizeCalls([
        { status: "error", error: refused },
        { status: "pending", error: null },
      ]),
    ).toEqual({ saveState: "saving", error: null });
    expect(
      summarizeCalls([
        { status: "pending", error: null },
        { status: "error", error: refused },
      ]),
    ).toEqual({ saveState: "saving", error: null });
  });

  it("shows the latest refusal, and says saved only when nothing was refused", () => {
    const later = new Error("later");
    expect(
      summarizeCalls([
        { status: "error", error: refused },
        { status: "error", error: later },
      ]).error,
    ).toBe(later);
    expect(
      summarizeCalls([
        { status: "success", error: null },
        { status: "success", error: null },
      ]),
    ).toEqual({ saveState: "saved", error: null });
    expect(summarizeCalls([])).toEqual({ saveState: "idle", error: null });
  });
});
