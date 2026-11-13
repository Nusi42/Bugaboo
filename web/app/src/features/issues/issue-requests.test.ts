import { afterEach, describe, expect, it, vi } from "vitest";

import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { counting, listRowsChanged, versionCounted, versionNow } from "./issue-requests.ts";
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
    expect(versionCounted(address, { version: 5, updatedAt: "2026-11-13T10:00:00Z" })).toBe(
      counting.inStep,
    );
    // A status change made right after is made on the new version, not on the old one.
    expect(versionNow(address, { version: 1 })).toBe(5);
  });

  it("never takes the version back when an older answer comes in late", () => {
    queryClient.setQueryData(queryKeys.issues.one(address), detail);
    versionCounted(address, { version: 5, updatedAt: "2026-11-13T10:00:05Z" });
    versionCounted(address, { version: 6, updatedAt: "2026-11-13T10:00:06Z" });
    expect(versionCounted(address, { version: 5, updatedAt: "2026-11-13T10:00:05Z" })).toBe(
      counting.late,
    );
    expect(versionNow(address, { version: 1 })).toBe(6);
  });

  it("leaves an issue that is not on screen alone", () => {
    versionCounted(address, { version: 5, updatedAt: "2026-11-13T10:00:00Z" });
    expect(queryClient.getQueryData(queryKeys.issues.one(address))).toBeUndefined();
  });

  it("reads the issue and the lists again when somebody else changed it in between", async () => {
    queryClient.setQueryData(queryKeys.issues.one(address), detail);
    const list = queryKeys.search.found("", "p1", "id,version");
    queryClient.setQueryData(list, { pages: [{ items: [] }], pageParams: [null] });
    // On screen is version 4; the tag made it 7, so 5 and 6 were made by somebody else: the page
    // must not take 7 as its own, or the next save would overwrite that change (ADR-0017).
    expect(versionCounted(address, { version: 7, updatedAt: "2026-11-13T10:00:00Z" })).toBe(
      counting.outOfStep,
    );
    expect(versionNow(address, { version: 1 })).toBe(4);
    await vi.waitFor(() => {
      expect(queryClient.getQueryState(queryKeys.issues.one(address))?.isInvalidated).toBe(true);
      expect(queryClient.getQueryState(list)?.isInvalidated).toBe(true);
    });
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
    listRowsChanged("i1", { version: 5, labels, updated_at: "2026-11-13T10:00:00Z" });
    expect(queryClient.getQueryData(key)).toEqual({
      pages: [{ items: [{ ...row, labels }, other] }],
      pageParams: [null],
    });
  });

  it("writes the version of a row that has one, and leaves a row that is newer already", () => {
    const key = queryKeys.search.found("", "p1", "id,version,labels");
    const older = { id: "i1", version: 4, labels: [] };
    const newer = { id: "i1", version: 6, labels: [] };
    queryClient.setQueryData(key, { pages: [{ items: [older] }, { items: [newer] }] });
    const labels = [{ id: "l1", name: "Urgent" }];
    listRowsChanged("i1", { version: 5, labels });
    expect(queryClient.getQueryData(key)).toEqual({
      pages: [{ items: [{ id: "i1", version: 5, labels }] }, { items: [newer] }],
    });
  });

  it("leaves a row without its version alone when the answer came late", () => {
    const key = queryKeys.search.found("", "p1", "id,labels");
    const row = { id: "i1", labels: [] };
    queryClient.setQueryData(key, { pages: [{ items: [row] }] });
    listRowsChanged("i1", { version: 5, labels: [{ id: "l1", name: "Urgent" }] }, true);
    expect(queryClient.getQueryData(key)).toEqual({ pages: [{ items: [row] }] });
  });
});
