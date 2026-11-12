import type { WordedChange } from "@bugaboo/wasm";
import { describe, expect, it } from "vitest";

import { renderSaid } from "./said.ts";
import { testWording } from "./wording-testing.ts";

/** A day as the change of a date carries it, worded for the region of the reader. */
function typed(value: string) {
  return { shown: "typed", kind: "date", value, worded: value } as const;
}

/** A date that moves from one day to another, the days worded for the region of the reader. */
function moved(field: string, before: string, after: string): WordedChange {
  return { who: "Lea", field, field_name: "A date", before: typed(before), after: typed(after) };
}

describe("plan_03 both dates moving in one event are one sentence, as in a notification", () => {
  const both = [moved("start-date", "Nov 3", "Nov 10"), moved("due-date", "Nov 7", "Nov 14")];

  it("says the event once, with the sentence the server words a notification with", () => {
    const said = testWording.changeSentences(both).map((each) => renderSaid(each));
    expect(said).toEqual(["Lea moved the dates from Nov 3–Nov 7 to Nov 10–Nov 14"]);
  });

  it("says a date that moves alone on its own", () => {
    const said = testWording
      .changeSentences([moved("due-date", "Nov 7", "Nov 14")])
      .map((each) => renderSaid(each));
    expect(said).toEqual(["Lea changed A date from Nov 7 to Nov 14"]);
  });
});
