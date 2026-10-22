import type { ReactNode } from "react";
import { SelectField, partOf, part, type ChangeStatus, type SelectChoice } from "@bugaboo/ui";

import { DetailRow } from "./DetailRow.tsx";
import { readOnlyReason } from "./read-only.ts";

export interface ChoiceRowProps<Value extends string> {
  readonly name: string;
  readonly choices: readonly SelectChoice<Value>[];
  readonly value: Value;
  /** Whether the person may change it in this issue. */
  readonly canChange: boolean;
  readonly status: ChangeStatus | undefined;
  readonly onPick: (value: Value) => void;
  /** What stands before the word or the list for a value: the icon of a priority (R5). */
  readonly lead?: (value: Value, label: string, showWord: boolean) => ReactNode;
}

/** A field of the Details with a fixed list of words (*Priority*): a list to pick from, or the word and why it is fixed. */
export function ChoiceRow<Value extends string>({
  name,
  choices,
  value,
  canChange,
  status,
  onPick,
  lead,
}: ChoiceRowProps<Value>) {
  if (!canChange) {
    const label = choices.find((choice) => choice.value === value)?.label ?? value;
    return (
      <DetailRow name={name} reason={readOnlyReason(name)}>
        {lead === undefined ? label : lead(value, label, true)}
      </DetailRow>
    );
  }
  return (
    <DetailRow name={name} status={status}>
      <div {...partOf(part.row)}>
        {lead === undefined
          ? null
          : lead(value, choices.find((choice) => choice.value === value)?.label ?? value, false)}
        <SelectField hideLabel label={name} choices={choices} value={value} onChange={onPick} />
      </div>
    </DetailRow>
  );
}
