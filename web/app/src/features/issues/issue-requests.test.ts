import { afterEach, describe, expect, it } from "vitest";

import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { rowChanged, versionCounted, versionNow } from "./issue-requests.ts";
import type { IssueDetail } from "./slots.ts";

const address = "WEB-142";

const detail: IssueDetail = {
  issue: { id: "i1", readable_id: address, title: "Login fails", version: 4 },
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
    versionCounted(address, { version: 5, updatedAt: "2026-11-13T10:00:00Z" });
    // A status change made right after is made on the new version, not on the old one.
    expect(versionNow(address, { version: 1 })).toBe(5);
  });

  it("never takes the version back when an older answer comes in late", () => {
    queryClient.setQueryData(queryKeys.issues.one(address), detail);
    versionCounted(address, { version: 6, updatedAt: "2026-11-13T10:00:06Z" });
    versionCounted(address, { version: 5, updatedAt: "2026-11-13T10:00:05Z" });
    expect(versionNow(address, { version: 1 })).toBe(6);
  });

  it("leaves an issue that is not on screen alone", () => {
    versionCounted(address, { version: 5, updatedAt: "2026-11-13T10:00:00Z" });
    expect(queryClient.getQueryData(queryKeys.issues.one(address))).toBeUndefined();
  });
});

describe("ux_6_4 a change in place shows its answer without reading the issue again", () => {
  it("writes when the issue was last changed, from the answer", () => {
    queryClient.setQueryData(queryKeys.issues.one(address), detail);
    versionCounted(address, { version: 5, updatedAt: "2026-11-13T10:00:00Z" });
    const known = queryClient.getQueryData<IssueDetail>(queryKeys.issues.one(address));
    expect(known?.issue.updated_at).toBe("2026-11-13T10:00:00Z");
  });

  it("changes the row of the issue in every list, only in the fields the row has", () => {
    const key = queryKeys.search.found("", "p1", "id,labels");
    const row = { id: "i1", title: "Login fails", labels: [] };
    const other = { id: "i2", title: "Other", labels: [] };
    queryClient.setQueryData(key, { pages: [{ items: [row, other] }], pageParams: [null] });
    const labels = [{ id: "l1", name: "Urgent" }];
    rowChanged("i1", { labels, updated_at: "2026-11-13T10:00:00Z" });
    expect(queryClient.getQueryData(key)).toEqual({
      pages: [{ items: [{ ...row, labels }, other] }],
      pageParams: [null],
    });
  });
});
