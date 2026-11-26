import { describe, expect, it } from "vitest";

import type { Release } from "../releases/index.ts";
import { marksOf } from "./marks.ts";

const project = { id: "p1", short_name: "WEB", name: "Website" } as const;

/** A release of the project WEB, as the page of releases reads it. */
function release(name: string, releaseDate: string | null, state: Release["state"]): Release {
  return {
    id: name,
    name,
    project,
    releaseDate,
    state,
    releasedOn: null,
    mayManage: true,
    version: 1,
  };
}

describe("plan_09_releases_and_goals_stand_on_their_dates_as_marks", () => {
  const window = { first: "2026-11-23", last: "2026-11-29" };

  it("puts a goal on its due date and a release on its release date, goals first", () => {
    const marks = marksOf(
      [{ issue: "i9", readable_id: "WEB-9", title: "Mobile app 2.0", day: "2026-11-26" }],
      [release("2.5", "2026-11-26", "planned")],
      window,
    );
    expect(marks.get("2026-11-26")?.map((mark) => [mark.kind, mark.name])).toEqual([
      ["goal", "Mobile app 2.0"],
      ["release", "2.5"],
    ]);
  });

  it("leaves out a release that is retired, has no date, or stands outside the days on screen", () => {
    const marks = marksOf(
      [],
      [
        release("2.3", "2026-11-26", "retired"),
        release("2.4", null, "planned"),
        release("2.6", "2026-12-03", "planned"),
        release("2.2", "2026-11-24", "released"),
      ],
      window,
    );
    expect([...marks.keys()]).toEqual(["2026-11-24"]);
  });

  it("names a goal the reader may not read the title of by its ID only", () => {
    const marks = marksOf(
      [{ issue: "i9", readable_id: "WEB-9", title: null, day: "2026-11-26" }],
      [],
      window,
    );
    expect(marks.get("2026-11-26")?.[0]?.name).toBeUndefined();
  });
});
