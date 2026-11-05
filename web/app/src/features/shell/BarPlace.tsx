import { Icon, type IconName, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

export interface BarPlaceProps {
  readonly icon: IconName;
  /** A number after the icon, such as the notifications not read yet. */
  readonly count?: string | undefined;
  /** The words of the place; the icon is drawn above them. */
  readonly children: ReactNode;
}

/**
 * The inside of a place of a bar of a phone (UX §3.3): the icon with its words under it. The words
 * come first in the page, so a screen reader says *Notifications 3*; the icon is drawn above them.
 * The bar of the app and the bar of an open issue draw their places with it.
 */
export function BarPlace({ icon, count, children }: BarPlaceProps) {
  return (
    <>
      <span {...partOf(part.bottomBarLabel)}>{children}</span>
      <span {...partOf(part.bottomBarIcon)}>
        <Icon name={icon} />
        {count === undefined ? null : <span {...partOf(part.bottomBarCount)}>{count}</span>}
      </span>
    </>
  );
}
