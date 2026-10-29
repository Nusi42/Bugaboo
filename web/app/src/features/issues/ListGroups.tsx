import { t } from "@bugaboo/messages";
import { part, partOf, Skeleton } from "@bugaboo/ui";

import type { SearchSpacing } from "../../shared/search-address.ts";
import { useFoundGroups } from "./found-issues.ts";
import { groupsOfKeys } from "./group-keys.ts";
import { GroupSection } from "./GroupSection.tsx";
import { useListContext } from "./list-context.ts";
import { ListEmpty } from "./ListEmpty.tsx";
import { ListScroll } from "./ListScroll.tsx";

export interface GroupedListProps {
  readonly spacing: SearchSpacing;
}

/** The groups of the search in the server's order, each with its count and its own rows (UX section 4.2). */
export function GroupedList({ spacing }: GroupedListProps) {
  const { search, groupBy, own } = useListContext();
  const groups = useFoundGroups(search, groupBy);
  if (groups.isPending) {
    return <Skeleton lines={3} />;
  }
  if (groups.data?.groups.length === 0) {
    return <ListEmpty />;
  }
  return (
    <>
      <ListScroll spacing={spacing} busy={groups.isFetching}>
        {groupsOfKeys(groups.data?.groups ?? [], groupBy === null ? undefined : own[groupBy]).map(
          (group) => (
            <GroupSection key={group.id} group={group} count={group.count} />
          ),
        )}
      </ListScroll>
      {groups.data?.truncated === true ? (
        <p {...partOf(part.muted)}>{t("issues.groups-cut")}</p>
      ) : null}
    </>
  );
}
