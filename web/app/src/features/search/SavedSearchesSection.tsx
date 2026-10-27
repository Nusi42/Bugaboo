import { t } from "@bugaboo/messages";
import { iconName, part, partOf } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import { itemsOf } from "../../shared/paged.ts";
import { paths, savedSearchAddress } from "../../shared/paths.ts";
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
      <ul {...partOf(part.list, { variants: ["tight"] })}>
        {searches.map((saved: SavedSearch) => (
          <li key={saved.id}>
            <NavLink
              icon={iconName.search}
              label={
                which === savedSet.shared
                  ? t("search.saved-by", { name: saved.name, person: personName(saved.maker) })
                  : saved.name
              }
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
 * *Saved searches* in the sidebar (UX design section 4.3): the page of *Search*, then the ones the
 * person made, then the ones shared with them, each with who made it. A saved search opens at its
 * own address.
 */
export function SavedSearchesSection({ narrow, onNavigate }: SavedSearchesSectionProps) {
  const { pathname } = useLocation();
  return (
    <ul {...partOf(part.sidebarList)}>
      <li>
        <NavLink
          icon={iconName.search}
          label={t("search.page.title")}
          narrow={narrow}
          href={paths.search}
          current={pathname === paths.search}
          {...(onNavigate === undefined ? {} : { onNavigate })}
        />
      </li>
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
