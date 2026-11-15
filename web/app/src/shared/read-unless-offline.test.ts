import { onlineManager } from "@tanstack/react-query";
import { doNothing } from "@bugaboo/ui";
import { afterEach, describe, expect, it } from "vitest";

import { readUnlessOffline } from "./read-unless-offline.ts";

/** A read that never answers, as one paused by the query cache while the browser is offline. */
function paused(): Promise<void> {
  return new Promise<void>(doNothing);
}

describe("iss_17 a read again after a save does not hold the editor while offline", () => {
  afterEach(() => {
    onlineManager.setOnline(true);
  });

  it("waits for the read while the browser is online", async () => {
    const order: string[] = [];
    await readUnlessOffline(async () => {
      await Promise.resolve();
      order.push("read");
    });
    expect(order).toEqual(["read"]);
  });

  it("stops waiting when the read fails, and leaves no failure behind", async () => {
    await expect(
      readUnlessOffline(() => Promise.reject(new Error("no connection"))),
    ).resolves.toBeUndefined();
  });

  it("does not wait when the browser is offline", async () => {
    onlineManager.setOnline(false);
    await expect(readUnlessOffline(paused)).resolves.toBeUndefined();
  });

  it("stops waiting when the browser goes offline during the read", async () => {
    const waiting = readUnlessOffline(paused);
    onlineManager.setOnline(false);
    await expect(waiting).resolves.toBeUndefined();
  });
});
