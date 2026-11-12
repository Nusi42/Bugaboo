import { t } from "@bugaboo/messages";
import { partOf, part, SelectField, type ColorToken } from "@bugaboo/ui";
import type { ColorRating, ShownColor } from "@bugaboo/wasm";

import { colorWords, groupLabels, type ColorGroup } from "./color-words.ts";
import { ColorRow } from "./ColorRow.tsx";
import { changedIn, colorsIn, isColorToken } from "./model.ts";
import { GroupSummary } from "./GroupSummary.tsx";
import type { ThemeValues } from "./themes.ts";

/** The value of the choice that changes no color yet: the list starts on it. */
const NONE = "";

export interface ColorGroupSectionProps {
  readonly group: ColorGroup;
  /** Whether the group is open when the editor opens; the person opens and closes it after. */
  readonly startsOpen: boolean;
  /** The colors of the group on screen: those the theme changes and those the person chose to change. */
  readonly shownTokens: readonly ColorToken[];
  /** Adds a color of the group to those on screen. */
  readonly onAdd: (token: ColorToken) => void;
  readonly values: ThemeValues;
  readonly colors: readonly ShownColor[];
  readonly ratings: readonly ColorRating[];
  readonly startsFrom: string;
  readonly accepts: (values: ThemeValues) => boolean;
  readonly onChange: (values: ThemeValues) => void;
}

/** *Change a color*: the colors of the group that are not on screen yet, by their plain names. */
function ChangeColor(props: {
  readonly others: readonly ColorToken[];
  readonly onAdd: (token: ColorToken) => void;
}) {
  const { others, onAdd } = props;
  return others.length === 0 ? null : (
    <SelectField
      label={t("theme-editor.change-color")}
      value={NONE}
      choices={[
        { value: NONE, label: t("theme-editor.change-color-choose") },
        ...others.map((token) => ({ value: token, label: colorWords[token].label })),
      ]}
      onChange={(chosen) => {
        if (isColorToken(chosen)) {
          onAdd(chosen);
        }
      }}
    />
  );
}

/**
 * A group of colors with its plain name (*Backgrounds*, *Text*, …), which opens and closes. It shows
 * only the colors the theme changes (UX §5.6), and *Change a color* adds another of the group.
 */
export function ColorGroupSection(props: ColorGroupSectionProps) {
  const { group, startsOpen, shownTokens, onAdd, colors, ...row } = props;
  return (
    <details {...partOf(part.themeGroup)} open={startsOpen}>
      <GroupSummary
        title={groupLabels[group]}
        note={t("theme-editor.group-changed", {
          count: changedIn(row.values, group).length,
          theme: row.startsFrom,
        })}
      />
      <div {...partOf(part.themeGroupBody)}>
        {shownTokens.map((token) => (
          <ColorRow
            key={token}
            token={token}
            shown={colors.find((color) => color.name === token)}
            {...row}
          />
        ))}
        <ChangeColor
          others={colorsIn(group).filter((token) => !shownTokens.includes(token))}
          onAdd={onAdd}
        />
      </div>
    </details>
  );
}
