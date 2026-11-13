import { buttonType, Icon, iconName, iconSize, part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";

export interface PhoneGroupProps {
  readonly open: boolean;
  readonly count: number;
  readonly onToggle: () => void;
  /** The name of the group: words, or a goal's Issue ID and title. */
  readonly children: ReactNode;
}

/**
 * The head of a group of the phone list, folded and opened like the head of a row on a computer;
 * *Not scheduled yet* has one like the others.
 */
export function PhoneGroup({ open, count, onToggle, children }: PhoneGroupProps) {
  return (
    <li {...partOf(part.timelinePhoneGroup)}>
      <button
        type={buttonType.button}
        aria-expanded={open}
        onClick={onToggle}
        {...partOf(part.timelineRowToggle)}
      >
        <Icon name={open ? iconName.down : iconName.next} size={iconSize.small} />
        {children}
        <span {...partOf(part.timelineRowCount)}>{count}</span>
      </button>
    </li>
  );
}
