import { beforeEach, describe, expect, it, vi } from "vitest";

const get = vi.fn<(path: string, options: unknown) => Promise<{ data?: unknown }>>();
vi.mock("../../shared/api.ts", () => ({
  api: { client: { GET: (path: string, options: unknown) => get(path, options) } },
}));

const { readIssueIds } = await import("./found-issues.ts");

/** A page of the search with these IDs, and the cursor of the next page if there is one. */
function page(ids: readonly string[], next?: string): { data: unknown } {
  return {
    data: {
      items: ids.map((id) => ({ id })),
      ...(next === undefined ? {} : { next_cursor: next }),
    },
  };
}

describe("tick all results reads the IDs of a search page after page", () => {
  beforeEach(() => {
    get.mockReset();
  });

  it("follows the cursor to the last page", async () => {
    get.mockResolvedValueOnce(page(["a", "b"], "c1")).mockResolvedValueOnce(page(["c"]));
    expect(await readIssueIds({ text: "x", within: undefined }, 5000)).toEqual(["a", "b", "c"]);
    expect(get).toHaveBeenCalledTimes(2);
    expect(JSON.stringify(get.mock.calls[1]?.[1])).toContain("c1");
  });

  it("stops at the most one change may name, and reads no further page", async () => {
    get.mockResolvedValueOnce(page(["a", "b", "c"], "c1")).mockResolvedValueOnce(page(["d"]));
    expect(await readIssueIds({ text: "", within: "p1" }, 2)).toEqual(["a", "b"]);
    expect(get).toHaveBeenCalledTimes(1);
  });
});
