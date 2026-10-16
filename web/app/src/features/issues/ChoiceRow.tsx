import { SelectField, type ChangeStatus, type SelectChoice } from "@bugaboo/ui";

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
}

/** A field of the Details with a fixed list of words (*Priority*): a list to pick from, or the word and why it is fixed. */
export function ChoiceRow<Value extends string>({
  name,
  choices,
  value,
  canChange,
  status,
  onPick,
}: ChoiceRowProps<Value>) {
  if (!canChange) {
    const current = choices.find((choice) => choice.value === value);
    return (
      <DetailRow name={name} reason={readOnlyReason(name)}>
        {current?.label ?? value}
      </DetailRow>
    );
  }
  return (
    <DetailRow name={name} status={status}>
      <SelectField hideLabel label={name} choices={choices} value={value} onChange={onPick} />
    </DetailRow>
  );
}
