import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import { queryKeys } from "../../shared/query-keys.ts";
import type { IssueBlockProps } from "../issues/index.ts";
import type { OwnField } from "./fields.ts";

/** An own field of issues that applies everywhere. */
function field(id: string, name: string): OwnField {
  return {
    applies_to: "issues",
    choices: [],
    help: "",
    id,
    kind: "text",
    look: "plain-text",
    may_manage: true,
    name,
    never_to_assistant: false,
    retired: false,
    rules: {},
    scope: { whole_organization: true, projects: [], issue_types: [] },
    version: 1,
  };
}

// The page the rows belong to reads a stored choice when it is loaded; there is no browser here.
vi.stubGlobal("localStorage", {
  getItem: vi.fn<(key: string) => string | null>(),
  setItem: vi.fn<(key: string, value: string) => void>(),
});

const { OwnFieldRows } = await import("./blocks.tsx");

describe("cfg_01 the own fields in the Details", () => {
  it("are drawn for an issue with no project, where no layout places them", () => {
    // The fields of the organization are in the cache, as when the page has read them.
    const cache = new QueryClient();
    cache.setQueryData(queryKeys.fields.list({}), {
      fields: [field("f1", "Alpha"), field("f2", "Beta")],
    });
    const props: IssueBlockProps = {
      issue: { id: "i1", own_fields: {} },
      address: "WEB-1",
      readableId: "WEB-1",
      ownFieldsHtml: [],
      mask: { sees: () => true, changes: () => false },
      form: { ready: true, failed: false, sections: [], shows: () => true },
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
