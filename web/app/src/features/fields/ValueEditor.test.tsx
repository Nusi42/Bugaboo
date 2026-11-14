import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import { field } from "./fields-testing.ts";

// The editors of people read a stored choice of the page when they are loaded; there is no browser here.
vi.stubGlobal("localStorage", {
  getItem: vi.fn<(key: string) => string | null>(),
  setItem: vi.fn<(key: string, value: string) => void>(),
});

const { ValueEditor } = await import("./ValueEditor.tsx");

/** Words of the tests, not text for people. */
const parts = { group: 'data-part="choice-group"', hidden: 'data-part="visually-hidden"' };

function ticks(inRow: boolean): string {
  return renderToStaticMarkup(
    <ValueEditor
      field={field({ kind: "pick-several" })}
      value={["minor"]}
      onCommit={vi.fn<(value: unknown) => void>()}
      inRow={inRow}
    />,
  );
}

describe("cf_03 the ticks of a field are the one group of choices", () => {
  it("stands in the shared frame of a group of choices, named by its first line", () => {
    const markup = ticks(false);
    expect(markup).toContain(parts.group);
    expect(markup).toContain('role="group"');
    expect(markup).toContain('data-part="choice-group.legend"');
  });

  it("names the group for a screen reader only in a row of the Details", () => {
    const markup = ticks(true);
    expect(markup).toContain(parts.hidden);
    expect(markup).not.toContain('data-part="choice-group.legend"');
  });
});
