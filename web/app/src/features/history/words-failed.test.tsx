import { t } from "@bugaboo/messages";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import { retryWording } from "../../shared/use-wording.ts";
import { ChangeEntry, WordsFailedNotice } from "./ChangeEntry.tsx";

// The module cannot be loaded: the network is gone.
vi.mock(import("@bugaboo/wasm"), async (importOriginal) => ({
  ...(await importOriginal()),
  loadWording: () => Promise.reject(new Error("offline")),
}));

const ben = { shown: "name", id: "b", name: "Ben" } as const;

/** The items of one event: two fields changed. */
const items = [
  { event: "e1", at: "2027-01-15T08:00:00Z", by: ben, kind: "change", field: "priority" },
  { event: "e1", at: "2027-01-15T08:00:00Z", by: ben, kind: "change", field: "status" },
] as const;

describe("ntf_02 a block of changes says once that its words could not be loaded", () => {
  it("leaves the failure out of every entry and says it once for the block, with Try again", async () => {
    retryWording();
    await vi.waitFor(() => {
      expect(renderToStaticMarkup(<WordsFailedNotice />)).not.toBe("");
    });
    const entries = renderToStaticMarkup(
      <>
        <ChangeEntry items={items} />
        <ChangeEntry items={items} />
      </>,
    );
    expect(entries).not.toContain(t("common.words-failed"));
    const block = renderToStaticMarkup(
      <>
        <WordsFailedNotice />
        {entries}
      </>,
    );
    expect(block.split(t("common.words-failed"))).toHaveLength(2);
    expect(block).toContain(t("common.try-again"));
  });
});
