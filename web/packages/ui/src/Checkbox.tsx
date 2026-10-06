import { CheckboxButton, CheckboxField } from "react-aria-components";

import styles from "./Checkbox.module.css";
import { cx } from "./cx.ts";

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
      <CheckboxButton className={cx(styles.checkbox)}>
        <span className={styles.box} />
        <span className={styles.label}>{label}</span>
      </CheckboxButton>
    </CheckboxField>
  );
}
