import { describe, expect, it } from "vitest";

import { queryKeys } from "./query-keys.ts";

describe("m3_f54 who changed a text is kept for one version of the issue", () => {
  it("has a key of its own for each issue, part and version, under the history", () => {
    const now = queryKeys.history.changedBy("ACME-1", "description", 4);
    expect(now).not.toEqual(queryKeys.history.changedBy("ACME-1", "description", 5));
    expect(now).not.toEqual(queryKeys.history.changedBy("ACME-1", "notes", 4));
    expect(now).not.toEqual(queryKeys.history.changedBy("ACME-2", "description", 4));
    // The key is not the one of a page of the history, so the two never share an answer.
    expect(now).not.toEqual(queryKeys.history.list("ACME-1", "changes"));
    // A change of the history reads it again with everything else under `history`.
    expect(now.slice(0, queryKeys.history.all.length)).toEqual(queryKeys.history.all);
  });
});
