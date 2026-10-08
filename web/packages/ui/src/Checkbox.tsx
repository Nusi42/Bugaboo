import { CheckboxButton, CheckboxField } from "react-aria-components";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface CheckboxProps {
  /** What ticking it means, as a sentence or a name, already in the person's language. */
  readonly label: string;
  readonly isSelected: boolean;
  readonly onChange: (isSelected: boolean) => void;
  readonly isDisabled?: boolean;
}

/** A box that is ticked or not, with its label next to it: one of a list of things to pick. */
export function Checkbox({ label, isSelected, onChange, isDisabled = false }: CheckboxProps) {
  return (
    <CheckboxField isSelected={isSelected} onChange={onChange} isDisabled={isDisabled}>
      <CheckboxButton {...partOf(part.checkbox)}>
        <span {...partOf(part.checkboxBox)} />
        <span {...partOf(part.checkboxLabel)}>{label}</span>
      </CheckboxButton>
    </CheckboxField>
  );
}
