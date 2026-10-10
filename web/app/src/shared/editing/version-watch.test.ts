import { describe, expect, it } from "vitest";

import { newerThan } from "./version-watch.ts";

const issue = { kind: "issue", id: "i1" };
const other = { kind: "issue", id: "i2" };

describe("adr_0019 what somebody else changed since the screen looked", () => {
  it("lists the records whose version is newer, with the version now", () => {
    const watched = [
      { record: issue, version: 3 },
      { record: other, version: 7 },
    ];
    const found = [
      { record: issue, version: 5 },
      { record: other, version: 7 },
    ];
    expect(newerThan(watched, found)).toEqual([{ record: issue, version: 5 }]);
  });

  it("leaves out a record that is gone or that the person may not know about", () => {
    const watched = [{ record: issue, version: 3 }];
    expect(newerThan(watched, [{ record: issue, version: null }])).toEqual([]);
    expect(newerThan(watched, [{ record: issue }])).toEqual([]);
    expect(newerThan(watched, [])).toEqual([]);
  });

  it("does not call an older or the same version a change", () => {
    const watched = [{ record: issue, version: 3 }];
    expect(newerThan(watched, [{ record: issue, version: 3 }])).toEqual([]);
    expect(newerThan(watched, [{ record: issue, version: 2 }])).toEqual([]);
  });
});
