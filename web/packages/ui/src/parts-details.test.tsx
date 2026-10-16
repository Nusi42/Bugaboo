import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Legend, LegendItem } from "./Legend.tsx";
import { NameValueList, NameValueRow } from "./NameValueRow.tsx";

const nothing = (): void => undefined;

/** Words of the tests, not text for people. */
const text = {
  table: "Issues",
  group: "In progress",
  name: "Assigned to",
  value: "Ana",
  saved: "Saved",
  undo: "Undo",
  key: "Key",
} as const;
const group = { inProgress: "in-progress" } as const;
const priority = { critical: "critical" } as const;

describe("ux_4_4 Details are name-and-value rows", () => {
  it("a row is a name and a value of a list of definitions", () => {
    const markup = renderToStaticMarkup(
      <NameValueList>
        <NameValueRow name={text.name}>{text.value}</NameValueRow>
      </NameValueList>,
    );
    expect(markup).toMatch(/<dl[^>]*data-part="name-values"/u);
    expect(markup).toMatch(/<dt[^>]*data-part="name-value\.name"[^>]*>Assigned to<\/dt>/u);
    expect(markup).toMatch(/<dd[^>]*data-part="name-value\.value"[^>]*>Ana/u);
  });

  it("says Saved with Undo where the change happened (R7)", () => {
    const saved = renderToStaticMarkup(
      <NameValueRow
        name={text.name}
        status={{
          state: "saved",
          labels: { saving: "Saving", saved: text.saved, "not-saved": "Not saved" },
          undoLabel: text.undo,
          onUndo: nothing,
        }}
      >
        {text.value}
      </NameValueRow>,
    );
    expect(saved).toContain(text.saved);
    expect(saved).toContain(text.undo);
    expect(saved).toContain('data-part="name-value.saved"');
  });

  it("shows no Undo before anything changed", () => {
    const quiet = renderToStaticMarkup(<NameValueRow name={text.name}>{text.value}</NameValueRow>);
    expect(quiet).not.toContain(text.undo);
  });
});

describe("ux_4_2 a legend says what the colors and the edge mean", () => {
  it("is a list with a name, and every item has a swatch and its words", () => {
    const markup = renderToStaticMarkup(
      <Legend label={text.table}>
        <LegendItem look={group.inProgress}>{text.group}</LegendItem>
        <LegendItem look={priority.critical}>{text.key}</LegendItem>
      </Legend>,
    );
    expect(markup).toContain('data-part="legend"');
    expect(markup).toContain(`aria-label="${text.table}"`);
    expect(markup).toContain('data-part="legend.swatch" data-variant="in-progress"');
    expect(markup).toContain('data-part="legend.swatch" data-variant="critical"');
    expect(markup).toContain(text.group);
  });
});
