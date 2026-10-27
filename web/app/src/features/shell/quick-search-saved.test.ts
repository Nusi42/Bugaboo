import { beforeEach, describe, expect, it, vi } from "vitest";

import { pageSize } from "../../shared/paged.ts";
import { asker } from "../../shared/quick-search-testing.ts";

const get = vi.fn<(path: string, options: unknown) => Promise<{ data?: unknown }>>();
vi.mock("../../shared/api.ts", () => ({
  api: { client: { GET: (path: string, options: unknown) => get(path, options) } },
}));

const { savedSearchesSource } = await import("./quick-search-saved.ts");

beforeEach(() => {
  get.mockReset();
});

describe("cfg_08 saved searches in quick search", () => {
  it("lists the ones the person made and the ones shared with them, whose name holds the words", async () => {
    // The two sets are asked in this order: the ones the person made, then the ones shared with them.
    get.mockResolvedValueOnce({
      data: {
        items: [
          { id: "a", name: "My open bugs" },
          { id: "b", name: "Done last week" },
        ],
      },
    });
    get.mockResolvedValueOnce({ data: { items: [{ id: "c", name: "Team bugs" }] } });
    const found = await savedSearchesSource.search("BUGS", asker);
    expect(found.map((one) => [one.id, one.path])).toEqual([
      ["a", "/s/a"],
      ["c", "/s/c"],
    ]);
    expect(found.map((one) => one.hint)).toEqual([undefined, "Shared with me"]);
    const route = "/api/v1/saved-searches";
    expect(get).toHaveBeenNthCalledWith(1, route, {
      params: { query: { scope: "mine", fields: "id,name", limit: pageSize.largest } },
    });
    expect(get).toHaveBeenNthCalledWith(2, route, {
      params: { query: { scope: "shared", fields: "id,name", limit: pageSize.largest } },
    });
  });

  it("asks nothing for an empty box", async () => {
    expect(await savedSearchesSource.search("", asker)).toEqual([]);
    expect(get).not.toHaveBeenCalled();
  });
});
