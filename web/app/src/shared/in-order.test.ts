import { beforeEach, describe, expect, it } from "vitest";

import { inOrder } from "./in-order.ts";

const log: string[] = [];

/** A job that ends when the test says so. */
class SlowJob {
  private release: () => void = () => {
    log.push("released before it was waited for");
  };
  private readonly waiting: Promise<void>;

  private readonly name: string;

  constructor(name: string) {
    this.name = name;
    this.waiting = new Promise<void>((resolve) => {
      this.release = resolve;
    });
  }

  finish(): void {
    this.release();
  }

  readonly run = async (): Promise<string> => {
    log.push(`${this.name} starts`);
    await this.waiting;
    log.push(`${this.name} ends`);
    return this.name;
  };
}

describe("adr_0019 changes of one record are sent one after the other", () => {
  beforeEach(() => {
    log.length = 0;
  });

  it("starts the second only when the first has ended", async () => {
    const first = new SlowJob("first");
    const second = new SlowJob("second");
    const a = inOrder("role-1", first.run);
    const b = inOrder("role-1", second.run);
    await Promise.resolve();
    expect(log).toEqual(["first starts"]);
    first.finish();
    await a;
    second.finish();
    await expect(b).resolves.toBe("second");
    expect(log).toEqual(["first starts", "first ends", "second starts", "second ends"]);
  });

  it("goes on after a job that failed, and passes its error on", async () => {
    const failing = inOrder("role-2", () => Promise.reject(new Error("refused")));
    const next = inOrder("role-2", () => Promise.resolve("fine"));
    await expect(failing).rejects.toThrow("refused");
    await expect(next).resolves.toBe("fine");
  });

  it("does not make records wait for each other", async () => {
    const held = new SlowJob("one");
    const waiting = inOrder("role-3", held.run);
    await expect(inOrder("role-4", () => Promise.resolve("other"))).resolves.toBe("other");
    held.finish();
    await waiting;
  });
});
