import { afterEach, describe, expect, it } from "vitest";

import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { versionCounted, versionNow } from "./issue-requests.ts";
import type { IssueDetail } from "./slots.ts";

const address = "WEB-142";

const detail: IssueDetail = {
  issue: { id: "i1", key: address, title: "Login fails", version: 4 },
  own_fields_html: [],
  may_change: true,
  may_delete: false,
  changeable: ["title", "status"],
};

afterEach(() => {
  queryClient.clear();
});

describe("adr_0017 a tag or label counts the version up and the page knows it at once", () => {
  it("writes the version the server counted to into the issue on screen", () => {
    queryClient.setQueryData(queryKeys.issues.one(address), detail);
    versionCounted(address, 5);
    // A status change made right after is made on the new version, not on the old one.
    expect(versionNow(address, { version: 1 })).toBe(5);
  });

  it("never takes the version back when an older answer comes in late", () => {
    queryClient.setQueryData(queryKeys.issues.one(address), detail);
    versionCounted(address, 6);
    versionCounted(address, 5);
    expect(versionNow(address, { version: 1 })).toBe(6);
  });

  it("leaves an issue that is not on screen alone", () => {
    versionCounted(address, 5);
    expect(queryClient.getQueryData(queryKeys.issues.one(address))).toBeUndefined();
  });
});
