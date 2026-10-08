import { SwitchButton, SwitchField } from "react-aria-components";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

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
      <SwitchButton {...partOf(part.switch)}>
        <span {...partOf(part.switchTrack)}>
          <span {...partOf(part.switchThumb)} />
        </span>
        {label}
      </SwitchButton>
    </SwitchField>
  );
}
