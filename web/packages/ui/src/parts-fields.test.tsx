import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { ChoiceGroup } from "./ChoiceGroup.tsx";
import { Link } from "./Link.tsx";
import { SelectField } from "./SelectField.tsx";
import { TextField } from "./TextField.tsx";
import { doNothing } from "./do-nothing.ts";

/** Words of the tests, not text for people. */
const text = { name: "Name", pick: "Pick", a: "A", b: "B", open: "Open", address: "/a" } as const;

describe("thm_12 the fields of packages/ui carry no class names and no styles", () => {
  it("a text field emits the parts of a field", () => {
    const markup = renderToStaticMarkup(
      <TextField label={text.name} value={text.address} onChange={doNothing} />,
    );
    for (const name of ["field", "field.label", "field.input"]) {
      expect(markup).toContain(`data-part="${name}"`);
    }
    expect(markup).not.toContain("style=");
  });
});

describe("thm_12 a label that is only read is hidden in one way", () => {
  it("is the part for text that is only read (M3-F187)", () => {
    const choices = [{ value: text.a, label: text.a }];
    const hidden = [
      renderToStaticMarkup(
        <TextField hideLabel label={text.name} value={text.address} onChange={doNothing} />,
      ),
      renderToStaticMarkup(
        <SelectField
          hideLabel
          label={text.name}
          choices={choices}
          value={text.a}
          onChange={doNothing}
        />,
      ),
    ];
    for (const markup of hidden) {
      expect(markup).toContain('data-part="visually-hidden"');
      expect(markup).not.toContain('data-part="field.label"');
      expect(markup).not.toContain("style=");
    }
  });

  it("a choice group and a link emit their parts", () => {
    const choices = renderToStaticMarkup(
      <ChoiceGroup
        legend={text.pick}
        choices={[
          { value: text.a, label: text.a },
          { value: text.b, label: text.b },
        ]}
        value={text.a}
        onChange={doNothing}
      />,
    );
    for (const name of ["choice-group", "choice-group.input", "choice-group.label"]) {
      expect(choices).toContain(`data-part="${name}"`);
    }
    const link = renderToStaticMarkup(<Link href={text.address}>{text.open}</Link>);
    expect(link).toContain('data-part="link"');
    expect(choices + link).not.toContain("style=");
  });
});
