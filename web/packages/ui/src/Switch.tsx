import { SwitchButton, SwitchField } from "react-aria-components";

import { cx } from "./cx.ts";
import styles from "./Switch.module.css";

export interface SwitchProps {
  /** What the switch does, as a sentence the person can say yes or no to. */
  readonly label: string;
  readonly isSelected: boolean;
  readonly onChange: (isSelected: boolean) => void;
}

/** An on and off switch, with a label next to it. */
export function Switch({ label, isSelected, onChange }: SwitchProps) {
  return (
    <SwitchField isSelected={isSelected} onChange={onChange}>
      <SwitchButton className={cx(styles.switch)}>
        <span className={styles.track}>
          <span className={styles.thumb} />
        </span>
        {label}
      </SwitchButton>
    </SwitchField>
  );
}
