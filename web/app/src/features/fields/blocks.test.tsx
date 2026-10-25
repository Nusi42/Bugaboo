import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import { queryKeys } from "../../shared/query-keys.ts";
import type { IssueBlockProps } from "../issues/index.ts";
import { field } from "./fields-testing.ts";

// The page the rows belong to reads a stored choice when it is loaded; there is no browser here.
vi.stubGlobal("localStorage", {
  getItem: vi.fn<(key: string) => string | null>(),
  setItem: vi.fn<(key: string, value: string) => void>(),
});

const { OwnFieldRows } = await import("./blocks.tsx");
const { formViewOf } = await import("../layouts/index.ts");

describe("cfg_01 the own fields in the Details", () => {
  it("are drawn for an issue with no project, where no layout places them", () => {
    // The fields of the organization are in the cache, as when the page has read them.
    const cache = new QueryClient();
    cache.setQueryData(queryKeys.fields.list({}), {
      fields: [field({ id: "f1", name: "Alpha" }), field({ id: "f2", name: "Beta" })],
    });
    const props: IssueBlockProps = {
      issue: { id: "i1", own_fields: {} },
      address: "WEB-1",
      readableId: "WEB-1",
      ownFieldsHtml: [],
      mask: { sees: () => true, changes: () => false },
      form: formViewOf({ data: undefined, isError: false }, false),
      version: 1,
      onChange: () => Promise.resolve(),
      rowStatus: vi.fn<IssueBlockProps["rowStatus"]>(),
      layout: "wide",
    };
    const html = renderToStaticMarkup(
      <QueryClientProvider client={cache}>
        <OwnFieldRows {...props} />
      </QueryClientProvider>,
    );
    expect(html.indexOf("Alpha")).toBeGreaterThanOrEqual(0);
    expect(html.indexOf("Beta")).toBeGreaterThan(html.indexOf("Alpha"));
  });
});
