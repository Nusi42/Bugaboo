import { describe, expect, it } from "vitest";

import { newBook } from "./book.ts";
import { counting, readsWentUp } from "./checks.ts";
import { parseOptions, QUIET_SECONDS } from "./options.ts";
import { pick, randomFrom } from "./random.ts";

describe("test_concurrent the driver of the load run reads its options", () => {
  it("runs ten people for two minutes on SQLite unless told otherwise", () => {
    expect(parseOptions([])).toEqual({ people: 10, seconds: 120, database: "sqlite", seed: 42 });
    expect(parseOptions(["--people", "4", "--seconds", "200", "--database", "postgres"])).toEqual({
      people: 4,
      seconds: 200,
      database: "postgres",
      seed: 42,
    });
  });

  it("refuses a run in which the locks cannot be seen to run out, and a wrong database", () => {
    expect(() => parseOptions(["--seconds", String(QUIET_SECONDS)])).toThrow(/at least/u);
    expect(() => parseOptions(["--database", "oracle"])).toThrow(/sqlite or postgres/u);
    expect(() => parseOptions(["--people", "0"])).toThrow(/whole number/u);
  });
});

/** Five choices from 0 up to 100. */
const run = (next: () => number) => Array.from({ length: 5 }, () => pick(next, 100));

describe("test_concurrent the same seed makes the same choices", () => {
  it("repeats for one seed, differs for another, and stays below the limit", () => {
    const first = randomFrom(7);
    const again = randomFrom(7);
    const other = randomFrom(8);
    expect(run(first)).toEqual(run(again));
    expect(run(randomFrom(7))).not.toEqual(run(other));
    expect(Math.max(...run(randomFrom(1)))).toBeLessThan(100);
    expect(Math.min(...run(randomFrom(1)))).toBeGreaterThanOrEqual(0);
  });
});

describe("test_concurrent the invariants of a run notice what is wrong", () => {
  it("counts the versions an issue must have gone through", () => {
    expect(counting(2, 5)).toEqual([2, 3, 4, 5]);
    expect(counting(2, 1)).toEqual([]);
  });

  it("finds a version that a person read after a newer one, and nothing else", () => {
    const book = newBook();
    book.note({ kind: "seen", person: 1, key: "A-1", version: 3 });
    book.note({ kind: "seen", person: 2, key: "A-1", version: 2 });
    book.note({ kind: "seen", person: 1, key: "A-1", version: 3 });
    expect(readsWentUp(book)).toEqual([]);
    book.note({ kind: "seen", person: 1, key: "A-1", version: 2 });
    expect(readsWentUp(book)).toHaveLength(1);
  });
});
