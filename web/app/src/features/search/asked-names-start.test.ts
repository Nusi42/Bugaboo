import { beforeEach, describe, expect, it, vi } from "vitest";

import { testSearchLanguage } from "../../shared/search-language-testing.ts";

const get = vi.fn<(path: string, options: unknown) => Promise<{ data: unknown }>>();
vi.mock("../../shared/api.ts", () => ({
  api: { client: { GET: (path: string, options: unknown) => get(path, options) } },
}));
// The finder reads a stored choice when it is loaded; there is no browser here.
vi.stubGlobal("localStorage", {
  getItem: vi.fn<(key: string) => string | null>(),
  setItem: vi.fn<(key: string, value: string) => void>(),
});

const { issuesStarting } = await import("./asked-names.ts");

beforeEach(() => {
  get.mockReset();
  get.mockResolvedValue({
    data: { items: [{ readable_id: "WEB-1", title: "Login fails after password reset" }] },
  });
});

describe("cfg_08 the Issue finder finds a title by the start of a word", () => {
  it("asks for the start of the word typed, so log finds the issue titled login", async () => {
    const found = await issuesStarting(testSearchLanguage, "log");
    expect(get).toHaveBeenCalledOnce();
    expect(get.mock.calls[0]?.[1]).toMatchObject({
      params: { query: { q: 'title contains "log*"' } },
    });
    expect(found.map((one) => one.value)).toEqual(["WEB-1"]);
  });

  it("keeps a finished word whole", async () => {
    await issuesStarting(testSearchLanguage, "log ");
    expect(get.mock.calls[0]?.[1]).toMatchObject({
      params: { query: { q: 'title contains "log"' } },
    });
  });

  it("asks nothing for a box of blanks", async () => {
    expect(await issuesStarting(testSearchLanguage, "   ")).toEqual([]);
    expect(get).not.toHaveBeenCalled();
  });
});
