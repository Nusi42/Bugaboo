import { Alert, alertTone } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import type { SearchSpacing } from "../../shared/search-address.ts";
import { linesFrom } from "../../shared/paged.ts";
import { foundLineOf, useFoundIssues } from "./found-issues.ts";
import { useListContext } from "./list-context.ts";
import { ListEmpty } from "./ListEmpty.tsx";
import { ListScroll } from "./ListScroll.tsx";
import { RowsOf } from "./ListRows.tsx";
import { useReportShown } from "./use-report-shown.ts";

export interface PlainListProps {
  readonly spacing: SearchSpacing;
}

/** The first group of a list that is not grouped has no name. */
const noGroup = "";

/** The issues the search finds, a page at a time, when the list is not grouped. */
export function PlainList({ spacing }: PlainListProps) {
  const context = useListContext();
  const found = useFoundIssues(context.search, context.words);
  const lines = linesFrom(found.data?.pages, foundLineOf);
  useReportShown("all", lines);
  if (found.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(found.error)}</Alert>;
  }
  if (found.isSuccess && lines.length === 0) {
    return <ListEmpty />;
  }
  return (
    <ListScroll spacing={spacing}>
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
