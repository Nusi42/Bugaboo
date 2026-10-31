import { Skeleton } from "@bugaboo/ui";

import { linesFrom } from "../../shared/paged.ts";
import { foundLineOf, useFoundIssues } from "./found-issues.ts";
import { useListContext } from "./list-context.ts";
import { ListEmpty } from "./ListEmpty.tsx";
import { ListScroll } from "./ListScroll.tsx";
import { RowsOf } from "./ListRows.tsx";
import { useReportShown } from "./use-report-shown.ts";

/** The first group of a list that is not grouped has no name. */
const noGroup = "";

/** The issues the search finds, a page at a time, when the list is not grouped. */
export function PlainList() {
  const context = useListContext();
  const found = useFoundIssues(context.search, context.words);
  const lines = linesFrom(found.data?.pages, foundLineOf);
  useReportShown("all", lines);
  // While the first answer is on its way the list shows that it is loading, not an empty frame.
  if (found.isPending) {
    return <Skeleton lines={3} />;
  }
  if (found.isSuccess && lines.length === 0) {
    return <ListEmpty />;
  }
  return (
    <ListScroll busy={found.isFetching && !found.isFetchingNextPage}>
      <RowsOf
        group={noGroup}
        lines={lines}
        hasMore={found.hasNextPage}
        isLoading={found.isFetchingNextPage}
        onMore={() => {
          if (!found.isFetchingNextPage) {
            void found.fetchNextPage();
          }
        }}
      />
    </ListScroll>
  );
}
