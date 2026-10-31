import { t } from "@bugaboo/messages";
import { Alert, alertTone, ShowMore, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { AssignmentRow } from "./AssignmentRow.tsx";
import { type RowLine, rowsOf, useAssignments } from "./assignments.ts";

export interface AssignmentListProps {
  /** The project the rows are for (an ID), or `null` for the whole organization. */
  readonly project: string | null;
  /** Takes a row away (the screen offers *Undo*). */
  readonly onRemove: (row: RowLine) => void;
  /** For a row from above, on a project: starts a row for this project for the same person or group. */
  readonly onChangeHere: ((row: RowLine) => void) | undefined;
}

/**
 * How many rows the list shows before *Show more*, on a phone and on a computer alike: a long list is a page of thousands
 * of pixels (M5-F135: a computer showed 21 rows of the whole organization at once).
 */
const SHOWN_ROWS = 10;

/** The rows of a place, a page at a time: those of the place first, then those that come from above. */
export function AssignmentList({ project, onRemove, onChangeHere }: AssignmentListProps) {
  const list = useAssignments(project);
  const all = rowsOf(list.data?.pages);
  const [shown, setShown] = useState(SHOWN_ROWS);
  const rows = all.slice(0, shown);
  const hiddenHere = all.length > shown;
  return (
    <>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {list.isSuccess && rows.length === 0 ? (
        <p {...partOf(part.muted)}>{t("assignments.none")}</p>
      ) : (
        <ul {...partOf(part.list)} aria-label={t("assignments.list")}>
          {rows.map((row) => (
            <AssignmentRow key={row.id} row={row} onRemove={onRemove} onChangeHere={onChangeHere} />
          ))}
        </ul>
      )}
      {hiddenHere || list.hasNextPage ? (
        <ShowMore
          label={t("assignments.more")}
          pending={list.isFetchingNextPage}
          onPress={() => {
            // Ten more are shown, and the next page is read where the rows that are loaded do not reach.
            setShown(shown + SHOWN_ROWS);
            if (all.length < shown + SHOWN_ROWS && list.hasNextPage) {
              void list.fetchNextPage();
            }
          }}
        />
      ) : null}
    </>
  );
}
