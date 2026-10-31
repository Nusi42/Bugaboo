import type { ReactNode } from "react";

import { Icon } from "./Icon.tsx";
import { Link } from "./Link.tsx";
import { iconName } from "./icons.ts";
import { iconSize } from "./props.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/** The sign that a row opens something: the one chevron, at the end of the row (M5 exit, O16). */
export function RowChevron() {
  return (
    <span {...partOf(part.rowChevron)}>
      <Icon name={iconName.next} size={iconSize.small} />
    </span>
  );
}

export interface LinkRowProps {
  /** Where the row leads. */
  readonly href: string;
  /** The name of what the row opens: the words of its link. */
  readonly children: ReactNode;
  /** What stands under or after the name: its details in quiet words. */
  readonly details?: ReactNode;
}

/**
 * A row of a list that opens a page (a group, a role, a field, a sub-project): its name is the link, the whole row can be
 * pressed (R8), its details follow the name, and the chevron stands at its end. The one way such a row is written.
 */
export function LinkRow({ href, children, details }: LinkRowProps) {
  return (
    <li {...partOf(part.item, { variants: ["row"] })}>
      <span {...partOf(part.rowMain)}>
        <Link row href={href}>
          {children}
        </Link>
        {details}
      </span>
      <RowChevron />
    </li>
  );
}
