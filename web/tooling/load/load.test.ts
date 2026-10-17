import { describe, expect, it } from "vitest";

import { newBook } from "./book.ts";
import { parseOptions, QUIET_SECONDS } from "./options.ts";
import { pick, randomFrom } from "./random.ts";

/** Where the run puts its data and its book, which every run must say. */
const where = ["--data-dir", "data", "--book", "book.json"];

describe("test_concurrent the driver of the load run reads its options", () => {
  it("runs ten people for two minutes on SQLite unless told otherwise", () => {
    expect(parseOptions(where)).toEqual({
      people: 10,
      seconds: 120,
      database: "sqlite",
      seed: 42,
      dataDir: "data",
      bookFile: "book.json",
    });
    expect(
      parseOptions(["--people", "4", "--seconds", "200", "--database", "postgres", ...where]),
    ).toEqual({
      people: 4,
      seconds: 200,
      database: "postgres",
      seed: 42,
      dataDir: "data",
      bookFile: "book.json",
    });
  });

  it("refuses a run in which the locks cannot be seen to run out, and a wrong database", () => {
    expect(() => parseOptions(["--seconds", String(QUIET_SECONDS), ...where])).toThrow(/at least/u);
    expect(() => parseOptions(["--database", "oracle", ...where])).toThrow(/sqlite or postgres/u);
    expect(() => parseOptions(["--people", "0", ...where])).toThrow(/whole number/u);
  });

  it("refuses a run that does not say where its data and its book go", () => {
    expect(() => parseOptions([])).toThrow(/--data-dir needs a value/u);
    expect(() => parseOptions(["--data-dir", "data"])).toThrow(/--book needs a value/u);
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

describe("test_concurrent the book of a run is what the people wrote", () => {
  it("holds every note in the file for bb-load-check and counts them for the summary", () => {
    const issue = { id: "00000000-0000-4000-8000-000000000001", key: "LOAD-1" };
    const book = newBook();
    book.kept({ key: "LOAD-1", version: 2 });
    book.kept({ key: "LOAD-1", version: 3, text: "A text that was kept" });
    book.kept({ key: "LOAD-1", version: 4, title: "A title", status: "in-progress" });
    book.comment("LOAD-1");
    book.fileAdded("LOAD-1");
    book.guest("00000000-0000-4000-8000-000000000004");
    book.hiddenFromGuests("00000000-0000-4000-8000-000000000005");
    book.seen({ person: 1, key: "LOAD-1", version: 2 });
    book.drafted({
      holder: "00000000-0000-4000-8000-000000000002",
      key: "LOAD-1",
      text: "A text that was refused",
      ending: "refused",
      found_as_draft: true,
    });
    book.drafted({
      holder: "00000000-0000-4000-8000-000000000003",
      key: "LOAD-1",
      text: "A text that was left behind",
      ending: "vanished",
    });
    const file = book.file([issue]);
    expect(file.issues).toEqual([issue]);
    expect(file.kept.map((note) => note.version)).toEqual([2, 3, 4]);
    expect(file.kept[2]).toMatchObject({ title: "A title", status: "in-progress" });
    expect(file.comments).toEqual(["LOAD-1"]);
    expect(file.files).toEqual(["LOAD-1"]);
    expect(file.guests).toEqual(["00000000-0000-4000-8000-000000000004"]);
    expect(file.hidden_from_guests).toEqual(["00000000-0000-4000-8000-000000000005"]);
    expect(file.seen).toHaveLength(1);
    expect(file.drafted.map((note) => note.ending)).toEqual(["refused", "vanished"]);
    expect(file.broken).toEqual([]);
    expect(book.counts()).toMatchObject({
      kept: 3,
      comments: 1,
      files: 1,
      refused: 1,
      vanished: 1,
    });
  });
});
