import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Alert } from "./Alert.tsx";
import { Button } from "./Button.tsx";
import { Checkbox } from "./Checkbox.tsx";
import { alertTone, buttonLook } from "./props.ts";
import { Switch } from "./Switch.tsx";
import { doNothing } from "./do-nothing.ts";

/** Words of the tests, not text for people. */
const text = { save: "Save", wrong: "Wrong", on: "On", yes: "Yes" } as const;

/** React Aria's default class names (`react-aria-Button`) are all that may stand in a markup. */
function ownClasses(markup: string): string[] {
  return (markup.match(/(?<=class=")[^"]*(?=")/gu) ?? []).filter(
    (classes) => !/^(react-aria-[A-Za-z]+ ?)*$/u.test(classes),
  );
}

describe("ux_4_4 a switch in a row of the Details keeps its name for screen readers only", () => {
  it("hides the label with the part for hidden text", () => {
    const markup = renderToStaticMarkup(
      <Switch hideLabel label={text.on} isSelected onChange={doNothing} />,
    );
    expect(markup).toContain(`data-part="visually-hidden">${text.on}<`);
  });
});

describe("thm_11 the controls of packages/ui carry no class names and no styles", () => {
  it("a button emits its part, its looks and its state", () => {
    const markup = renderToStaticMarkup(
      <Button look={buttonLook.primary} wide isPending>
        {text.save}
      </Button>,
    );
    expect(markup).toContain('data-part="button"');
    expect(markup).toContain('data-variant="primary wide"');
    expect(markup).toContain('data-state="busy"');
    expect(markup).not.toContain("style=");
    expect(ownClasses(markup)).toEqual([]);
  });

  it("an alert emits its tone as the variant", () => {
    const markup = renderToStaticMarkup(<Alert tone={alertTone.danger}>{text.wrong}</Alert>);
    expect(markup).toContain('data-part="alert"');
    expect(markup).toContain('data-variant="danger"');
    expect(ownClasses(markup)).toEqual([]);
  });

  it("a switch and a checkbox emit the parts they are made of", () => {
    const toggle = renderToStaticMarkup(<Switch label={text.on} isSelected onChange={doNothing} />);
    for (const name of ["switch", "switch.track", "switch.thumb"]) {
      expect(toggle).toContain(`data-part="${name}"`);
    }
    // A real switch: assistive technology says "on" and "off", not "checked".
    expect(toggle).toContain('role="switch"');
    const box = renderToStaticMarkup(
      <Checkbox label={text.yes} isSelected={false} onChange={doNothing} />,
    );
    for (const name of ["checkbox", "checkbox.box", "checkbox.label"]) {
      expect(box).toContain(`data-part="${name}"`);
    }
    expect(ownClasses(toggle + box)).toEqual([]);
  });
});
