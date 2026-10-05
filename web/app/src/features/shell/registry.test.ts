import { describe, expect, it } from "vitest";

import { createRegistry } from "./registry.ts";

describe("ux_01 what features add to the shell", () => {
  it("lists what was added, in order, and forgets what was taken out", () => {
    const registry = createRegistry<{ readonly name: string }>();
    const first = { name: "first" };
    const second = { name: "second" };
    const removeFirst = registry.add(first);
    registry.add(second);
    expect(registry.list()).toEqual([first, second]);
    removeFirst();
    expect(registry.list()).toEqual([second]);
  });

  it("tells those who show the list about every change", () => {
    const registry = createRegistry<{ readonly name: string }>();
    let heard = 0;
    const stop = registry.subscribe(() => {
      heard += 1;
    });
    const remove = registry.add({ name: "x" });
    remove();
    expect(heard).toBe(2);
    stop();
    registry.add({ name: "y" });
    expect(heard).toBe(2);
  });
});
