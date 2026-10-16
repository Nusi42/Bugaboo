import { partOf } from "./parts.ts";
import { iconSize } from "./props.ts";
import type { StatusGroup } from "./status.ts";
import { StatusIcon } from "./StatusIcon.tsx";
import { part } from "./tokens.ts";

export interface StatusPillProps {
  readonly group: StatusGroup;
  /** The word of the status, in the person's language: a project's own status keeps its group's color. */
  readonly label: string;
}

/** The small pill that shows a status in lists and in text: the group's icon and the word. */
export function StatusPill({ group, label }: StatusPillProps) {
  return (
    <span {...partOf(part.statusPill, { variants: [group] })}>
      <StatusIcon group={group} size={iconSize.small} />
      {label}
    </span>
  );
}
