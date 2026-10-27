import { describe, expect, it } from "vitest";

import { inOrderOfKey } from "./change-my-settings.ts";

function held() {
  const gate = Promise.withResolvers<void>();
  return { promise: gate.promise, release: gate.resolve };
}

describe("ux_04 requests that change one setting", () => {
  it("go one after the other, whatever the first one takes", async () => {
    const seen: string[] = [];
    const first = held();
    const a = inOrderOfKey("k", async () => {
      seen.push("a start");
      await first.promise;
      seen.push("a end");
    });
    const b = inOrderOfKey("k", () => {
      seen.push("b start");
      return Promise.resolve();
    });
    await Promise.resolve();
    expect(seen).toEqual(["a start"]);
    first.release();
    await Promise.all([a, b]);
    expect(seen).toEqual(["a start", "a end", "b start"]);
  });

  it("still go when the one before was refused, and tell each its own answer", async () => {
    const a = inOrderOfKey("k2", () => Promise.reject(new Error("no")));
    const b = inOrderOfKey("k2", () => Promise.resolve("yes"));
    await expect(a).rejects.toThrow("no");
    await expect(b).resolves.toBe("yes");
  });

  it("do not wait for requests of another setting", async () => {
    const slow = held();
    void inOrderOfKey("k3", () => slow.promise);
    await expect(inOrderOfKey("k4", () => Promise.resolve(1))).resolves.toBe(1);
    slow.release();
  });
});
