import { t } from "@bugaboo/messages";
import { Button, buttonLook, FieldValue, partOf, part } from "@bugaboo/ui";
import { useRef } from "react";

import { removeKeepingFocus } from "../../shared/focus.ts";
import { chipLook, type Chip } from "./labels.ts";

export interface ChipsProps {
  /** The name of the list for people who read it with a screen reader. */
  readonly label: string;
  readonly chips: readonly Chip[];
  /** What to say when there are none. */
  readonly none: string;
  /** Takes a chip off and says whether it went; the buttons are there only where the person may change the issue. */
  readonly onTakeOff?: ((chip: Chip) => Promise<boolean>) | undefined;
}

/** The tags or labels of an issue side by side, each with the button that takes it off. */
export function Chips({ label, chips, none, onTakeOff }: ChipsProps) {
  const list = useRef<HTMLUListElement>(null);
  if (chips.length === 0) {
    return <p {...partOf(part.muted)}>{none}</p>;
  }
  return (
    <ul ref={list} {...partOf(part.chips)} aria-label={label}>
      {chips.map((chip, index) => (
        <li key={chip.id} {...partOf(part.chip)}>
          <FieldValue look={chipLook} text={chip.name} color={chip.color} />
          {onTakeOff === undefined ? null : (
            <Button
              look={buttonLook.quiet}
              aria-label={t("labels.take-off", { name: chip.name })}
              onPress={() => {
                // The button goes with its chip: the focus moves to the next one, or to adding one.
                const row = list.current?.children[index];
                removeKeepingFocus(
                  () => onTakeOff(chip),
                  row instanceof HTMLElement ? row : undefined,
                );
              }}
            >
              {t("labels.take-off-short")}
            </Button>
          )}
        </li>
      ))}
    </ul>
  );
}
