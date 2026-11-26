import { t } from "@bugaboo/messages";
import { iconName } from "@bugaboo/ui";

import { itemsOf } from "../../shared/paged.ts";
import { savedSearchAddress } from "../../shared/paths.ts";
import { madeListShown, MineAndShared, type SidebarSectionView } from "../shell/index.ts";
import { savedHeading, useSavedList, savedSet } from "./saved-searches.ts";

export interface SavedSearchesSectionProps {
  readonly view: SidebarSectionView;
}

/**
 * *Saved searches* in the sidebar (UX design section 4.3): the ones the person made, then the ones
 * shared with them, each with who made it, in two sub-groups. The page of *Search* is not one of
 * them: it stands with the start page above. A saved search opens at its own address.
 */
export function SavedSearchesSection({ view }: SavedSearchesSectionProps) {
  const mine = useSavedList(savedSet.mine, madeListShown);
  const shared = useSavedList(savedSet.shared, madeListShown);
  return (
    <MineAndShared
      view={view}
      heading={savedHeading}
      icon={iconName.search}
      mine={{ ...mine, items: itemsOf(mine.data?.pages) }}
      shared={{ ...shared, items: itemsOf(shared.data?.pages) }}
      addressOf={savedSearchAddress}
      none={t("search.saved-none")}
    />
  );
}
