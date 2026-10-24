import { CancelledError } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { currentMySettings, mySettingsKey } from "./my-settings.ts";
import { queryClient } from "./query-client.ts";
import { leaveSession } from "./session.ts";

/** A pause long enough for the page to have changed and nothing else to have happened. */
function pause(): Promise<void> {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, 20);
  });
}

describe("sec_12 leaving the session does not cancel the questions on their way", () => {
  beforeEach(() => {
    // The storage of the browser, which forgetting the choices of the person touches.
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: vi.fn<() => void>(),
      removeItem: vi.fn<() => void>(),
    });
  });

  it("lets a question that is on its way be answered, and empties the cache after", async () => {
    queryClient.clear();
    const gate = Promise.withResolvers<string>();
    const asked = queryClient.query({ queryKey: ["on-its-way"], queryFn: () => gate.promise });
    const left = leaveSession(() => Promise.resolve());
    // The page has changed and the question is still on its way: the cache is not emptied yet.
    await pause();
    expect(queryClient.getQueryCache().findAll()).toHaveLength(1);
    gate.resolve("the answer");
    await expect(asked).resolves.toBe("the answer");
    await left;
    expect(queryClient.getQueryCache().findAll()).toHaveLength(0);
  });

  it("stops reading the look from the settings of the person as soon as the page has changed", async () => {
    queryClient.clear();
    queryClient.setQueryData(mySettingsKey(), { settings: [] });
    const gate = Promise.withResolvers<string>();
    const asked = queryClient.query({ queryKey: ["waiting"], queryFn: () => gate.promise });
    const left = leaveSession(() => Promise.resolve());
    await pause();
    // A theme chosen on the sign-in page now must not go to the person who has left, nor be drawn
    // over again with their look when the cache is emptied.
    expect(currentMySettings()).toBeUndefined();
    gate.resolve("the answer");
    await asked;
    await left;
  });

  it("is what cancels that question when the cache is emptied under it", async () => {
    queryClient.clear();
    const gate = Promise.withResolvers<string>();
    const asked = queryClient.query({ queryKey: ["cancelled"], queryFn: () => gate.promise });
    queryClient.clear();
    await expect(asked).rejects.toBeInstanceOf(CancelledError);
  });
});
