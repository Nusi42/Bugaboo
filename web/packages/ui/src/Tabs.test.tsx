import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Tabs } from "./Tabs.tsx";

/** Words of the tests, not text for people. */
const text = {
  list: "What to show",
  one: "One",
  two: "Two",
  inOne: "In one",
  inTwo: "In two",
} as const;
const ids = { one: "one", two: "two" } as const;

describe("ux_4_4 tabs are one component", () => {
  const markup = renderToStaticMarkup(
    <Tabs
      label={text.list}
      tabs={[
        { id: ids.one, label: text.one },
        { id: ids.two, label: text.two },
      ]}
      selected={ids.two}
      onSelect={(): void => undefined}
    >
      {(id) => (id === ids.one ? text.inOne : text.inTwo)}
    </Tabs>,
  );

  it("names the list, draws every tab, and shows only the panel of the open one", () => {
    expect(markup).toContain(`aria-label="${text.list}"`);
    expect(markup).toContain('data-part="tabs"');
    expect(markup).toContain(text.one);
    expect(markup).toContain(text.two);
    expect(markup).toContain(text.inTwo);
    expect(markup).not.toContain(text.inOne);
  });

  it("carries only parts, no class names of its own", () => {
    expect(markup).toContain('data-part="tab"');
    expect(markup).toContain('data-part="tab-panel"');
  });
});
