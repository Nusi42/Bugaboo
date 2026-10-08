import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

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

/** The rows of a place, a page at a time: those of the place first, then those that come from above. */
export function AssignmentList({ project, onRemove, onChangeHere }: AssignmentListProps) {
  const list = useAssignments(project);
  const rows = rowsOf(list.data?.pages);
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
      {list.hasNextPage ? (
        <div {...partOf(part.row)}>
          <Button
            isPending={list.isFetchingNextPage}
            onPress={() => {
              void list.fetchNextPage();
            }}
          >
            {t("assignments.more")}
          </Button>
        </div>
      ) : null}
    </>
  );
}
