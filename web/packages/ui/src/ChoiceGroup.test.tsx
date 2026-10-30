import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { ChoiceGroup } from "./ChoiceGroup.tsx";
import { doNothing } from "./do-nothing.ts";
import { choiceGroupLook } from "./props.ts";

/** Words of the tests, not text for people. */
const words = { legend: "Color", one: "Pink", two: "Blue", swatch: "swatch" };
const choices = [
  { value: "cat1", label: words.one },
  { value: "cat2", label: words.two },
] as const;

describe("ux_5 the one choice group, also for the colors", () => {
  it("lays many small choices in the grid look and draws each with the label it is given", () => {
    const markup = renderToStaticMarkup(
      <ChoiceGroup
        legend={words.legend}
        choices={choices}
        value={undefined}
        look={choiceGroupLook.grid}
        renderLabel={(choice) => <i data-test={words.swatch}>{choice.label}</i>}
        onChange={doNothing}
      />,
    );
    expect(markup).toContain('data-variant="grid"');
    expect(markup).toContain(`data-test="${words.swatch}"`);
    expect(markup).not.toContain("checked");
  });
});
