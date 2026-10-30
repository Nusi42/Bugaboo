import { t } from "@bugaboo/messages";
import { iconName, part, partOf } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import { itemsOf } from "../../shared/paged.ts";
import { savedSearchAddress } from "../../shared/paths.ts";
import { personName } from "../../shared/person.ts";
import { NavLink } from "../shell/index.ts";
import { useSavedList, savedSet, type SavedSearch, type SavedSet } from "./saved-searches.ts";

export interface SavedSearchesSectionProps {
  readonly narrow: boolean;
  readonly onNavigate?: (() => void) | undefined;
}

/** How many saved searches of one list the sidebar shows; the rest is on the page of *Search*. */
const shown = 20;

/** One list of saved searches, with who made each of the ones shared with the person. */
function SavedList({
  which,
  heading,
  pathname,
  onNavigate,
}: Omit<SavedSearchesSectionProps, "narrow"> & {
  readonly which: SavedSet;
  readonly heading: string;
  readonly pathname: string;
}) {
  const list = useSavedList(which, shown);
  const searches = itemsOf(list.data?.pages);
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
              current={pathname === savedSearchAddress(saved.id)}
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
export function SavedSearchesSection({ narrow, onNavigate }: SavedSearchesSectionProps) {
  const { pathname } = useLocation();
  return (
    <ul {...partOf(part.sidebarList)}>
      {narrow ? null : (
        <>
          <SavedList
            which={savedSet.mine}
            heading={t("search.saved-mine")}
            pathname={pathname}
            onNavigate={onNavigate}
          />
          <SavedList
            which={savedSet.shared}
            heading={t("search.saved-shared")}
            pathname={pathname}
            onNavigate={onNavigate}
          />
        </>
      )}
    </ul>
  );
}
