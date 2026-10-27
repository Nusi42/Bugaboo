import { t } from "@bugaboo/messages";
import { Checkbox, IconButton, iconName, part, partOf } from "@bugaboo/ui";

import { keptWords } from "../../shared/list-columns.ts";

export interface ColumnRowProps {
  /** The search word of the column. */
  readonly word: string;
  readonly name: string;
  /** The column is ticked and stands among the columns, in this place. */
  readonly place?: { readonly index: number; readonly last: number } | undefined;
  readonly onTick: (on: boolean) => void;
  readonly onMove: (direction: "up" | "down") => void;
}

/** A field in *Customize › Columns*: a box to tick, and for a ticked one the buttons that move it. */
export function ColumnRow({ word, name, place, onTick, onMove }: ColumnRowProps) {
  return (
    <li {...partOf(part.listPanelItem)}>
      <Checkbox
        label={name}
        isSelected={place !== undefined}
        isDisabled={keptWords.includes(word)}
        onChange={onTick}
      />
      {place === undefined ? null : (
        <span {...partOf(part.row)}>
          <IconButton
            icon={iconName.up}
            label={t("issues.column-up", { column: name })}
            isDisabled={place.index === 0}
            onPress={() => {
              onMove("up");
            }}
          />
          <IconButton
            icon={iconName.down}
            label={t("issues.column-down", { column: name })}
            isDisabled={place.index === place.last}
            onPress={() => {
              onMove("down");
            }}
          />
        </span>
      )}
    </li>
  );
}
