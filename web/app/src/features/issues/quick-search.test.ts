import { ApiError } from "@bugaboo/api-client";
import type { Search } from "@bugaboo/wasm-search";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { asker } from "../../shared/quick-search-testing.ts";

/**
 * The search language is the module of Rust, whose tests start it (`web/tooling`); here the
 * printer only says what it was given, so this source is tested without the module.
 */
const print = vi.fn<(search: Search) => string>(() => 'title contains "login"');
vi.mock("@bugaboo/wasm-search", () => ({
  loadSearchLanguage: () => Promise.resolve({ print, openLastWord: (text: string) => text }),
}));

const get = vi.fn<(path: string, options: unknown) => Promise<{ data?: unknown }>>();
vi.mock("../../shared/api.ts", () => ({
  api: { client: { GET: (path: string, options: unknown) => get(path, options) } },
}));

const { issuesSource, looksLikeIssueId } = await import("./quick-search.ts");

const ISSUE = "/api/v1/issues/{issue}";
const SEARCH = "/api/v1/search/issues";

/**
 * The answers of the server by the route that was asked. An answer that is an error is what the
 * client does with a refusal: it throws, it does not return an empty answer.
 */
function answers(byRoute: Readonly<Record<string, unknown>>): void {
  get.mockImplementation((path) => {
    const answer = byRoute[path];
    return answer instanceof Error ? Promise.reject(answer) : Promise.resolve({ data: answer });
  });
}

beforeEach(() => {
  get.mockReset();
  print.mockClear();
});

describe("cfg_08 an ID opens the issue", () => {
  it("cfg_08_an_id_opens_the_issue: asks for the issue of the ID and goes to its page", async () => {
    answers({
      [ISSUE]: { issue: { readable_id: "WEB-12", title: "Login fails" } },
      [SEARCH]: { items: [] },
    });
    const found = await issuesSource.search("WEB-12", asker);
    expect(found).toEqual([
      { id: "WEB-12", title: "Login fails", hint: "WEB-12", path: "/issues/WEB-12" },
    ]);
    expect(get).toHaveBeenCalledWith(ISSUE, { params: { path: { issue: "WEB-12" } } });
  });

  it("shows nothing for an ID of an issue that is hidden or missing: the two look the same", async () => {
    answers({ [ISSUE]: new ApiError(404, undefined), [SEARCH]: { items: [] } });
    expect(await issuesSource.search("WEB-404", asker)).toEqual([]);
  });

  it("keeps the issues of the words when the text is no ID the server knows (e-mail)", async () => {
    answers({
      [ISSUE]: new ApiError(400, undefined),
      [SEARCH]: { items: [{ readable_id: "WEB-3", title: "E-mail bounces" }] },
    });
    expect((await issuesSource.search("e-mail", asker)).map((one) => one.id)).toEqual(["WEB-3"]);
  });

  it("is a broken source when the server fails, not when it finds nothing", async () => {
    answers({ [ISSUE]: new ApiError(500, undefined), [SEARCH]: { items: [] } });
    await expect(issuesSource.search("web-1", asker)).rejects.toBeInstanceOf(ApiError);
  });

  it("does not repeat the issue of the ID among the issues of the words", async () => {
    const same = { readable_id: "WEB-12", title: "WEB-12 fix" };
    answers({ [ISSUE]: { issue: same }, [SEARCH]: { items: [same] } });
    expect(await issuesSource.search("web-12", asker)).toHaveLength(1);
  });

  it("tells an ID from words", () => {
    expect(looksLikeIssueId("web-142")).toBe(true);
    expect(looksLikeIssueId(" WEB-K7Q2X ")).toBe(true);
    expect(looksLikeIssueId("login page")).toBe(false);
    expect(looksLikeIssueId("sign-in page")).toBe(false);
    expect(looksLikeIssueId("")).toBe(false);
  });
});

describe("cfg_08 words find issues by title", () => {
  it("cfg_08_words_find_issues_by_title: searches the title only, ten issues at most, and lists them", async () => {
    answers({
      [SEARCH]: {
        items: [
          { readable_id: "WEB-1", title: "Login fails" },
          { readable_id: "WEB-2" },
          { readable_id: "API-7", title: "Login slow" },
        ],
      },
    });
    const found = await issuesSource.search("login", asker);
    expect(found.map((one) => one.path)).toEqual(["/issues/WEB-1", "/issues/API-7"]);
    expect(get).toHaveBeenCalledOnce();
    expect(get).toHaveBeenCalledWith(SEARCH, {
      params: { query: { q: 'title contains "login"', fields: "readable_id,title", limit: 10 } },
    });
    expect(print.mock.calls[0]?.[0]).toMatchObject({
      filter: {
        field: { word: "title" },
        test: { type: "contains", text: { text: "login" } },
      },
    });
  });

  it("finds nothing, and is not broken, for a text the checker of the language refuses", async () => {
    answers({ [SEARCH]: new ApiError(400, undefined) });
    expect(await issuesSource.search("many words", asker)).toEqual([]);
  });

  it("asks nothing for an empty box", async () => {
    expect(await issuesSource.search("  ", asker)).toEqual([]);
    expect(get).not.toHaveBeenCalled();
  });
});
