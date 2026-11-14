import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { ChoiceGroup, ChoiceGroupFrame } from "./ChoiceGroup.tsx";
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

  it("shows each choice of the radios look with its round mark, the chosen one checked", () => {
    const markup = renderToStaticMarkup(
      <ChoiceGroup
        legend={words.legend}
        choices={choices}
        value={choices[0].value}
        look={choiceGroupLook.radios}
        onChange={doNothing}
      />,
    );
    expect(markup).toContain('data-variant="radios"');
    expect(markup.match(/data-part="choice-group\.dot"/gu)).toHaveLength(2);
    expect(markup.match(/checked=""/gu)).toHaveLength(1);
  });

  it("draws no round mark in the usual look", () => {
    const markup = renderToStaticMarkup(
      <ChoiceGroup
        legend={words.legend}
        choices={choices}
        value={choices[0].value}
        onChange={doNothing}
      />,
    );
    expect(markup).not.toContain("choice-group.dot");
  });
});

describe("ux_6_4 a group of choices a page can set its name beside other things", () => {
  it("is a group named by its first line, not a fieldset with a legend", () => {
    const markup = renderToStaticMarkup(
      <ChoiceGroup
        legend={words.legend}
        choices={choices}
        value={choices[0].value}
        onChange={doNothing}
      />,
    );
    const name = /id="(?<id>[^"]+)" data-part="choice-group\.legend"/u.exec(markup)?.groups?.["id"];
    expect(markup).toContain('role="group"');
    expect(markup).toContain(`aria-labelledby="${String(name)}"`);
    expect(markup).not.toContain("<fieldset");
  });
});

describe("ux_6_4 the one frame of a group of choices, whatever its choices are", () => {
  it("names any choices it holds by its first line, with no fieldset or legend", () => {
    const markup = renderToStaticMarkup(
      <ChoiceGroupFrame legend={words.legend}>
        <i>{words.one}</i>
      </ChoiceGroupFrame>,
    );
    const name = /id="(?<id>[^"]+)" data-part="choice-group\.legend"/u.exec(markup)?.groups?.["id"];
    expect(markup).toContain('data-part="choice-group"');
    expect(markup).toContain(`aria-labelledby="${String(name)}"`);
    expect(markup).toContain(words.one);
    expect(markup).not.toContain("<fieldset");
    expect(markup).not.toContain("<legend");
  });

  it("keeps the name for a screen reader only when the row shows it already", () => {
    const markup = renderToStaticMarkup(
      <ChoiceGroupFrame legend={words.legend} hideLegend>
        {words.one}
      </ChoiceGroupFrame>,
    );
    const name = /id="(?<id>[^"]+)" data-part="visually-hidden"/u.exec(markup)?.groups?.["id"];
    expect(markup).toContain(`aria-labelledby="${String(name)}"`);
    expect(markup).not.toContain("choice-group.legend");
  });
});
