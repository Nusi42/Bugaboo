import type { ColorToken } from "@bugaboo/ui";

import { colorGroups } from "./color-words.ts";
import { ColorGroupSection } from "./ColorGroupSection.tsx";
import { ShapeGroup } from "./ShapeGroup.tsx";
import { startsFromName } from "./theme-facts.ts";
import { shownIn, startsOpen } from "./use-shown-colors.ts";
import type { Editing } from "./use-theme-editing.ts";

/** The color a new layer of a shadow starts with: the dimmed page of the theme. */
const SHADE: ColorToken = "scrim";

export interface ColorGroupsProps {
  readonly editing: Editing;
  /** The ID of the built-in theme the own theme starts from. */
  readonly startsFrom: string;
}

/**
 * The groups of the editor (UX §5.6): *Backgrounds*, *Text*, *Lines*, *Highlight color*, *Status
 * and tag colors*, each with the colors the theme changes and those the person chose to change,
 * then *Shadow, corners and fonts*.
 */
export function ColorGroups({ editing, startsFrom }: ColorGroupsProps) {
  const { values, saved, shown } = editing;
  const named = startsFromName(startsFrom);
  return (
    <>
      {colorGroups.map((group) => (
        <ColorGroupSection
          key={group}
          group={group}
          startsOpen={startsOpen(saved, group)}
          shownTokens={shownIn(values, group, shown.added)}
          onAdd={shown.reveal}
          values={values}
          colors={editing.read?.colors ?? []}
          pairs={editing.read?.pairs ?? []}
          startsFrom={named}
          accepts={editing.accepts}
          onChange={editing.setValues}
        />
      ))}
      <ShapeGroup
        values={values}
        startsFrom={named}
        shade={editing.read?.colors.find((color) => color.name === SHADE)?.value ?? ""}
        accepts={editing.accepts}
        onChange={editing.setValues}
      />
    </>
  );
}
