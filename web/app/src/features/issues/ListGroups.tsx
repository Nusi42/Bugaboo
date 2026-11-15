import { t } from "@bugaboo/messages";
import { part, partOf, DelayedSkeleton, useHoldWhileReading } from "@bugaboo/ui";

import { useFoundGroups } from "./found-issues.ts";
import { GroupSection } from "./GroupSection.tsx";
import { useListContext } from "./list-context.ts";
import { ListEmpty } from "./ListEmpty.tsx";
import { ListScroll } from "./ListScroll.tsx";
import { useReadyGroups } from "./use-ready-groups.ts";

/** The groups of the search in the server's order, each with its count and its own rows (UX section 4.2). */
export function GroupedList() {
  const { search, groupBy } = useListContext();
  const found = useFoundGroups(search, groupBy);
  const ready = useReadyGroups(found.isPlaceholderData ? undefined : found.data);
  // The groups wait for their first answer, and then for the first rows of the groups at the top, which end even when
  // they cannot be read: a list held with them shows heads and rows together (UX §6.4).
  useHoldWhileReading(found.isLoading || (ready.groups === undefined && found.data !== undefined));
  if (ready.groups === undefined) {
    return <DelayedSkeleton lines={3} />;
  }
  if (ready.empty) {
    return <ListEmpty />;
  }
  return (
    <>
      <ListScroll busy={found.isFetching}>
        {ready.groups.map((group) => (
          <GroupSection key={group.id} group={group} count={group.count} />
        ))}
      </ListScroll>
      {ready.truncated ? <p {...partOf(part.muted)}>{t("issues.groups-cut")}</p> : null}
    </>
  );
}
