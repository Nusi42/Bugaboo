import { t } from "@bugaboo/messages";
import { iconName, part, partOf } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { itemsOf } from "../../shared/paged.ts";
import { savedSearchAddress } from "../../shared/paths.ts";
import { personName } from "../../shared/person.ts";
import { howCurrent, placeCurrent } from "../../shared/places.ts";
import { NavLink, SectionFrame, type SidebarSectionView } from "../shell/index.ts";
import {
  savedHeading,
  useSavedList,
  savedSet,
  type SavedSearch,
  type SavedSet,
} from "./saved-searches.ts";

export interface SavedSearchesSectionProps {
  readonly view: SidebarSectionView;
}

/** How many saved searches of one list the sidebar shows; the rest is on the page of *Search*. */
const shown = 20;

/** One list of saved searches, with who made each of the ones shared with the person. */
function SavedList({
  which,
  searches,
  heading,
  pathname,
  onNavigate,
}: {
  readonly onNavigate?: (() => void) | undefined;
  readonly which: SavedSet;
  readonly searches: readonly SavedSearch[];
  readonly heading: string;
  readonly pathname: string;
}) {
  if (searches.length === 0) {
    return null;
  }
  return (
    <li>
      <p {...partOf(part.navSubheading)}>{heading}</p>
      <ul {...partOf(part.navSubgroup)}>
        {searches.map((saved: SavedSearch) => (
          <li key={saved.id}>
            <NavLink
              icon={iconName.search}
              label={saved.name}
              {...(which === savedSet.shared
                ? {
                    by: personName(saved.maker),
                    bySeparator: t("search.saved-by-separator"),
                  }
                : {})}
              href={savedSearchAddress(saved.id)}
              current={howCurrent(savedSearchAddress(saved.id), pathname) === placeCurrent.page}
              {...(onNavigate === undefined ? {} : { onNavigate })}
            />
          </li>
        ))}
      </ul>
    </li>
  );
}

/**
 * *Saved searches* in the sidebar (UX design section 4.3): the ones the person made, then the ones
 * shared with them, each with who made it, in two sub-groups. The page of *Search* is not one of
 * them: it stands with the start page above. A saved search opens at its
 * own address.
 */
export function SavedSearchesSection({ view }: SavedSearchesSectionProps) {
  const { narrow, onNavigate } = view;
  const { pathname } = useLocation();
  const mineList = useSavedList(savedSet.mine, shown);
  const sharedList = useSavedList(savedSet.shared, shown);
  const mine = itemsOf(mineList.data?.pages);
  const shared = itemsOf(sharedList.data?.pages);
  const framed = (children: ReactNode) => (
    <SectionFrame heading={savedHeading} view={view}>
      {children}
    </SectionFrame>
  );
  // While the lists are read the section says so in one line, so the menu does not grow under the person.
  if (!narrow && (mineList.isPending || sharedList.isPending)) {
    return framed(<p {...partOf(part.sidebarNote)}>{t("common.loading")}</p>);
  }
  const none = mineList.isSuccess && sharedList.isSuccess && mine.length + shared.length === 0;
  // A person without a saved search sees no heading and no note in the sidebar: a window 600 px high has no
  // room for them (M5-F150). The sheet *More* of a phone has room and tells it.
  if (!narrow && none) {
    return view.place === "sheet"
      ? framed(<p {...partOf(part.sidebarNote)}>{t("search.saved-none")}</p>)
      : null;
  }
  return framed(
    <ul {...partOf(part.sidebarList)}>
      {narrow ? null : (
        <>
          <SavedList
            which={savedSet.mine}
            searches={mine}
            heading={t("search.saved-mine")}
            pathname={pathname}
            onNavigate={onNavigate}
          />
          <SavedList
            which={savedSet.shared}
            searches={shared}
            heading={t("search.saved-shared")}
            pathname={pathname}
            onNavigate={onNavigate}
          />
        </>
      )}
    </ul>,
  );
}
