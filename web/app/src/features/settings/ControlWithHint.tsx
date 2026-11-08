import { partOf, part, Switch } from "@bugaboo/ui";
import type { ReactNode } from "react";

/**
 * A control with its hint under it, as close as a box and its hint: the one frame of the hint of every
 * setting that is not a box typed in (a switch, choices, a page's own control).
 */
export function ControlWithHint({
  help,
  children,
}: {
  readonly help: string;
  readonly children: ReactNode;
}) {
  return (
    <div {...partOf(part.settingsControl)}>
      {children}
      <p {...partOf(part.settingsHelp)}>{help}</p>
    </div>
  );
}

export interface SwitchBoxProps {
  readonly label: string;
  readonly isSelected: boolean;
  readonly onChange: (selected: boolean) => void;
  /** The hint under the switch. */
  readonly help: string;
  /** What stands in the box under the hint (a warning, a way back to the standard). */
  readonly children?: ReactNode;
}

/** A switch in a box of its own with its hint, like the switches of the settings forms. */
export function SwitchBox({ label, isSelected, onChange, help, children }: SwitchBoxProps) {
  return (
    <div {...partOf(part.box)}>
      <ControlWithHint help={help}>
        <Switch label={label} isSelected={isSelected} onChange={onChange} />
      </ControlWithHint>
      {children}
    </div>
  );
}
