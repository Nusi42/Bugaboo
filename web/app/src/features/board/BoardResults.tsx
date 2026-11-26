import { t } from "@bugaboo/messages";
import { Alert, alertTone, part, partOf, useLayout, DelayedSkeleton } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { useOwnColumns } from "../issues/index.ts";
import { BoardBody } from "./BoardBody.tsx";
import { BoardHead } from "./BoardHead.tsx";
import type { BoardResultsProps } from "./results-props.ts";
import { useBoard } from "./use-board.ts";
import { choicesOf, ownByIdOf } from "./own-fields.ts";

/**
 * *Show as › Board* (UX §4.6, PLAN-06): the tools, the Legend, and the columns of cards the board
 * route answers for the reader (ADR-0030): the screen holds no rule of the workflow. A card moves by
 * a drag, by keys or by *Move to …*; on a phone one column shows at a time.
 */
export function BoardResults(props: BoardResultsProps) {
  const { text, within, view, fields } = props;
  const phone = useLayout() === "phone";
  const own = useOwnColumns(props.language, true);
  const { read, model } = useBoard({ text, within, options: view.board });
  const shown = {
    search: { text, within },
    own,
    ownOf: ownByIdOf(own, fields.names),
    order: { language: props.language, text, onText: props.onText, names: fields.names },
    options: view.board,
    phone,
  };
  return (
    <div {...partOf(part.board)}>
      <BoardHead
        view={view}
        onView={props.onView}
        fields={fields}
        columns={model?.board.columns ?? []}
        keptIn={props.keptIn}
        choices={choicesOf(fields, own)}
        phone={phone}
      />
      {read.isError && model === undefined ? (
        <Alert tone={alertTone.danger}>
          {t("board.read-failed")} {messageOf(read.error)}
        </Alert>
      ) : null}
      {model === undefined ? (
        read.isError ? null : (
          <DelayedSkeleton lines={3} />
        )
      ) : (
        <BoardBody model={model} {...shown} />
      )}
      {model?.problem === undefined || model.problem === null ? null : (
        <Alert tone={alertTone.danger}>{model.problem}</Alert>
      )}
    </div>
  );
}
