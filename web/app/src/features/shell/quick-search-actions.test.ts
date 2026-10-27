import { describe, expect, it, vi } from "vitest";

import { askerFor } from "../../shared/quick-search-testing.ts";
import { actionsSource, addQuickAction } from "./quick-search-actions.ts";

const sign = vi.fn<() => void>();
const asker = askerFor(sign);

describe("cfg_08 the actions of quick search", () => {
  it("offers Sign out, which signs out", async () => {
    const found = await actionsSource.search("sign", asker);
    expect(found.map((one) => one.title)).toEqual(["Sign out"]);
    found[0]?.run?.();
    expect(sign).toHaveBeenCalledOnce();
    expect(found[0]?.path).toBeUndefined();
  });

  it("offers what a feature added, found by any word of its name", async () => {
    const run = vi.fn<() => void>();
    const remove = addQuickAction({ id: "create-issue", title: "Create issue", run });
    const found = await actionsSource.search("issue create", asker);
    expect(found.map((one) => one.id)).toEqual(["create-issue"]);
    remove();
    expect(await actionsSource.search("issue", asker)).toEqual([]);
  });

  it("offers nothing for words no action holds", async () => {
    expect(await actionsSource.search("zebra", asker)).toEqual([]);
  });
});
