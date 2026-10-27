import { t } from "@bugaboo/messages";
import { Alert, alertTone, part, partOf } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import type { SearchSpacing } from "../../shared/search-address.ts";
import { useFoundGroups } from "./found-issues.ts";
import { groupOfKey } from "./group-keys.ts";
import { GroupSection } from "./GroupSection.tsx";
import { useListContext } from "./list-context.ts";
import { ListEmpty } from "./ListEmpty.tsx";
import { ListScroll } from "./ListScroll.tsx";

export interface GroupedListProps {
  readonly spacing: SearchSpacing;
}

/** The groups of the search in the server's order, each with its count and its own rows (UX section 4.2). */
export function GroupedList({ spacing }: GroupedListProps) {
  const { search, groupBy } = useListContext();
  const groups = useFoundGroups(search, groupBy);
  if (groups.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(groups.error)}</Alert>;
  }
  if (groups.data?.groups.length === 0) {
    return <ListEmpty />;
  }
  return (
    <>
      <ListScroll spacing={spacing}>
        {(groups.data?.groups ?? []).map((each) => {
          const group = groupOfKey(each, groupBy ?? "");
          return <GroupSection key={group.id} group={group} count={each.issues} />;
        })}
      </ListScroll>
      {groups.data?.truncated === true ? (
        <p {...partOf(part.muted)}>{t("issues.groups-cut")}</p>
      ) : null}
    </>
  );
}
