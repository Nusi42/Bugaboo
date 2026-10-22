import { Button } from "react-aria-components";

import { Icon } from "./Icon.tsx";
import { iconName } from "./icons.ts";
import { partOf } from "./parts.ts";
import { iconSize } from "./props.ts";
import type { StatusGroup } from "./status.ts";
import { StatusIcon } from "./StatusIcon.tsx";
import { part } from "./tokens.ts";

export interface StatusButtonProps {
  readonly group: StatusGroup;
  /** The word of the current status. */
  readonly label: string;
  /** What a press does; a computer's list opens under the button by itself, so it is given none there. */
  readonly onPress?: (() => void) | undefined;
  readonly isDisabled?: boolean;
  /** The ID of the button, so a key or the bar of a phone can take the person to it. */
  readonly id?: string;
}

/**
 * The big button of the issue page: the current status with its group's icon and color, and an
 * arrow that says it can be changed (UX §4.4). What it offers is for the screen to decide.
 */
export function StatusButton({ group, label, onPress, isDisabled = false, id }: StatusButtonProps) {
  return (
    <Button
      {...partOf(part.statusButton, { variants: [group] })}
      {...(id === undefined ? {} : { id })}
      {...(onPress === undefined ? {} : { onPress })}
      isDisabled={isDisabled}
    >
      <StatusIcon group={group} />
      {label}
      <Icon name={iconName.down} size={iconSize.small} />
    </Button>
  );
}
