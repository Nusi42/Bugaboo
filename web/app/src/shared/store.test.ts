import { describe, expect, it } from "vitest";

import { createStore } from "./store.ts";

describe("a store", () => {
  it("tells_its_listeners_when_the_state_is_replaced_and_not_after_they_stop", () => {
    const store = createStore(1);
    let told = 0;
    const stop = store.subscribe(() => {
      told += 1;
    });
    store.replace(2);
    expect(store.snapshot()).toBe(2);
    expect(told).toBe(1);
    stop();
    store.replace(3);
    expect(told).toBe(1);
  });
});
