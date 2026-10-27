import { t } from "@bugaboo/messages";
import { buttonType, Icon, iconName, iconSize, part, partOf, StatusIcon } from "@bugaboo/ui";
import { GridListHeader } from "react-aria-components";

import type { GroupOfKey } from "./group-keys.ts";

/** What a group is called in its header. */
function nameOf(group: GroupOfKey): string {
  if (group.statusGroup !== undefined) {
    return t(`status-group.${group.statusGroup}`);
  }
  if (group.priority !== undefined) {
    return t(`priority.${group.priority}`);
  }
  if (group.name !== undefined) {
    return group.name;
  }
  return group.filter === undefined ? t("issues.group-unnamed") : t("issues.group-empty");
}

export interface GroupHeaderProps {
  readonly group: GroupOfKey;
  readonly count: number;
  readonly open: boolean;
  readonly onToggle: () => void;
  /** Why the group shows no rows, if it cannot. */
  readonly note: string | undefined;
}

/** The header of a group (UX section 4.2): its icon and color, its name and how many issues it holds; it folds the group. */
export function GroupHeader({ group, count, open, onToggle, note }: GroupHeaderProps) {
  return (
    <GridListHeader {...partOf(part.listGroup)}>
      <button
        type={buttonType.button}
        {...partOf(part.listGroupToggle)}
        aria-expanded={open}
        onClick={onToggle}
      >
        <Icon name={open ? iconName.down : iconName.next} size={iconSize.small} />
        {group.statusGroup === undefined ? null : (
          <StatusIcon group={group.statusGroup} size={iconSize.small} />
        )}
        <span {...partOf(part.listGroupName)}>{nameOf(group)}</span>
        <span {...partOf(part.listGroupCount)}>{count}</span>
        {note === undefined ? null : <span {...partOf(part.listGroupNote)}>{note}</span>}
      </button>
    </GridListHeader>
  );
}
