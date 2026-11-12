import { t } from "@bugaboo/messages";
import { Icon, iconName, iconSize, part, partOf, Priority, StatusIcon } from "@bugaboo/ui";
import { GridListItem } from "react-aria-components";

import { formatNumber } from "../../shared/format.ts";
import { groupName, type GroupOfKey } from "./group-keys.ts";
import { useListContext } from "./list-context.ts";

export interface GroupHeaderProps {
  readonly group: GroupOfKey;
  readonly count: number;
  readonly open: boolean;
  readonly onToggle: () => void;
  /** Why the group shows no rows, if it cannot. */
  readonly note: string | undefined;
}

/**
 * The header of a group (UX section 4.2): its icon and color, its name and how many issues it holds.
 * It is a row of the list that is reached with the arrow keys like every other row, and pressing it
 * (Enter, Space or a click) folds the group. It cannot be ticked.
 */
export function GroupHeader({ group, count, open, onToggle, note }: GroupHeaderProps) {
  const name = groupName(group);
  const { ticks } = useListContext();
  const itemId = `${group.id}/header`;
  const words = t(open ? "issues.group-open" : "issues.group-folded", { group: name, count });
  return (
    <GridListItem
      id={itemId}
      textValue={words}
      aria-label={words}
      isDisabled
      onAction={onToggle}
      {...partOf(part.listGroup)}
    >
      <span {...partOf(part.listGroupToggle, { variants: [!ticks && "no-select"] })}>
        <span {...partOf(part.listGroupChevron)}>
          <Icon name={open ? iconName.down : iconName.next} size={iconSize.small} />
        </span>
        <span {...partOf(part.listGroupKind)}>
          {group.statusGroup === undefined ? null : (
            <StatusIcon group={group.statusGroup} size={iconSize.small} />
          )}
          {group.priority === undefined ? null : (
            <Priority
              priority={group.priority}
              label={t(`priority.${group.priority}`)}
              showWord={false}
            />
          )}
        </span>
        <span {...partOf(part.listGroupName)}>{name}</span>
        <span {...partOf(part.listGroupCount)}>{formatNumber(count)}</span>
        {note === undefined ? null : <span {...partOf(part.listGroupNote)}>{note}</span>}
      </span>
    </GridListItem>
  );
}
