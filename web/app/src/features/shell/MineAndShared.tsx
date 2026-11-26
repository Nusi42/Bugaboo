import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { part, partOf, useHoldWhileReading, useShortWindow, type IconName } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { personName } from "../../shared/person.ts";
import { howCurrent, placeCurrent } from "../../shared/places.ts";
import { NavLink } from "./NavItem.tsx";
import { SectionFrame } from "./SectionFrame.tsx";
import type { SidebarSectionView } from "./sidebar-sections.ts";

/** Something a person made and may share: a saved search or a dashboard. */
export interface MadeThing {
  readonly id: string;
  readonly name: string;
  readonly maker: Schema<"PersonRef">;
}

/** One of the two lists as it is read: its items, and whether it is read yet. */
export interface MadeList {
  readonly items: readonly MadeThing[];
  readonly isPending: boolean;
  readonly isLoading: boolean;
  readonly isSuccess: boolean;
}

/** One list of the section, with who made each of the ones shared with the person. */
function MadeLinks({
  heading,
  things,
  shared,
  icon,
  addressOf,
  pathname,
  onNavigate,
}: {
  readonly heading: string;
  readonly things: readonly MadeThing[];
  readonly shared: boolean;
  readonly icon: IconName;
  readonly addressOf: (id: string) => string;
  readonly pathname: string;
  readonly onNavigate: (() => void) | undefined;
}) {
  if (things.length === 0) {
    return null;
  }
  return (
    <li>
      <p {...partOf(part.navSubheading)}>{heading}</p>
      <ul {...partOf(part.navSubgroup)}>
        {things.map((thing) => (
          <li key={thing.id}>
            <NavLink
              icon={icon}
              label={thing.name}
              {...(shared
                ? { by: personName(thing.maker), bySeparator: t("common.saved-by-separator") }
                : {})}
              href={addressOf(thing.id)}
              current={howCurrent(addressOf(thing.id), pathname) === placeCurrent.page}
              {...(onNavigate === undefined ? {} : { onNavigate })}
            />
          </li>
        ))}
      </ul>
    </li>
  );
}

/** Whether both lists are read and hold nothing. */
function isEmpty(mine: MadeList, shared: MadeList): boolean {
  return mine.isSuccess && shared.isSuccess && mine.items.length + shared.items.length === 0;
}

/** An empty section: the line that says so, and what it offers to make. */
function EmptyLists({ none, after }: { readonly none: string; readonly after: ReactNode }) {
  return (
    <>
      <p {...partOf(part.sidebarNote)}>{none}</p>
      {after === undefined ? null : <ul {...partOf(part.sidebarList)}>{after}</ul>}
    </>
  );
}

/** *Mine*, then *Shared with me*. */
function BothLists({
  mine,
  shared,
  ...rest
}: {
  readonly mine: readonly MadeThing[];
  readonly shared: readonly MadeThing[];
  readonly icon: IconName;
  readonly addressOf: (id: string) => string;
  readonly pathname: string;
  readonly onNavigate: (() => void) | undefined;
}) {
  return (
    <>
      <MadeLinks heading={t("common.saved-mine")} things={mine} shared={false} {...rest} />
      <MadeLinks heading={t("common.saved-shared")} things={shared} shared {...rest} />
    </>
  );
}

export interface MineAndSharedProps {
  readonly view: SidebarSectionView;
  /** The heading of the section: *Saved searches*, *Dashboards*. */
  readonly heading: MessageKey;
  readonly icon: IconName;
  /** The things the person made, and the ones shared with them. */
  readonly mine: MadeList;
  readonly shared: MadeList;
  /** Where a thing opens: its own address. */
  readonly addressOf: (id: string) => string;
  /** The line an empty section shows: *You have no saved searches yet.* */
  readonly none: string;
  /** What stands under the lists: *Create a dashboard*. */
  readonly after?: ReactNode;
}

/**
 * A section of the sidebar with the things a person made and the ones shared with them, in two sub-groups *Mine* and
 * *Shared with me*, each shared one with who made it (UX §4.3, §4.10): *Saved searches* and *Dashboards*. While the
 * lists are read the section says so in one line, so the menu does not grow under the person.
 */
export function MineAndShared({
  view,
  heading,
  icon,
  mine,
  shared,
  addressOf,
  none,
  after,
}: MineAndSharedProps) {
  const { narrow, onNavigate } = view;
  const { pathname } = useLocation();
  const low = useShortWindow();
  useHoldWhileReading(mine.isLoading || shared.isLoading);
  const framed = (children: ReactNode) => (
    <SectionFrame heading={heading} view={view}>
      {children}
    </SectionFrame>
  );
  if (!narrow && (mine.isPending || shared.isPending)) {
    return framed(<p {...partOf(part.sidebarNote)}>{t("common.loading")}</p>);
  }
  // An empty section keeps its heading and says so in one line, like every section. Only in a window too low for the
  // sidebar (the height that folds its bottom group) it is the first thing left out, with what it offers to make: the
  // sidebar must fit a window 600 px high (M5-F150, UX §4.9, M10-F130); the sheet *More* of a phone always has room.
  if (!narrow && isEmpty(mine, shared)) {
    if (low && view.place !== "sheet") {
      return null;
    }
    return framed(<EmptyLists none={none} after={after} />);
  }
  return framed(
    <ul {...partOf(part.sidebarList)}>
      {narrow ? null : (
        <BothLists
          mine={mine.items}
          shared={shared.items}
          icon={icon}
          addressOf={addressOf}
          pathname={pathname}
          onNavigate={onNavigate}
        />
      )}
      {after}
    </ul>,
  );
}
