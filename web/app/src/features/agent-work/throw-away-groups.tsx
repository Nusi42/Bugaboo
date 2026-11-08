import { t } from "@bugaboo/messages";
import { IssueId, type LongPreviewGroup } from "@bugaboo/ui";
import type { Wording } from "@bugaboo/wasm";

import { changeSidesText } from "../../shared/change-line.ts";
import { skipLineText, type RunChange, type UndoSkip } from "../runs/index.ts";
import { staysField } from "./throw-away-stays.ts";

/**
 * What goes back, item by item: the issue is the group, each field that changes back is a line that
 * says what will happen (*Priority goes back from Critical to Normal*). A field the preview names
 * among what stays is not here; it is in the lines of `stayGroups`.
 */
export function goesBackGroups(
  wording: Wording,
  items: readonly Readonly<RunChange>[],
  skipped: readonly UndoSkip[],
): readonly LongPreviewGroup[] {
  return items.flatMap((item) => {
    const fields = item.changes.filter((field) => !staysField(skipped, item.issue.id, field.field));
    return fields.length === 0
      ? []
      : [
          {
            key: item.event,
            heading: <IssueId id={item.issue.readable_id} />,
            lines: fields.map((field) => {
              const sides = changeSidesText(wording, field);
              return {
                key: field.field,
                // The change went from before to after; going back is the other way round.
                content: t("agent-work.throw-away-back", {
                  field: sides.field,
                  from: sides.after,
                  to: sides.before,
                }),
              };
            }),
          },
        ];
  });
}

/** What stays of one skipped field: one line each, said as the page of the run says it. */
export function stayGroups(skipped: readonly UndoSkip[]): readonly LongPreviewGroup[] {
  return skipped.map((skip, at) => ({
    key: ["stay", skip.issue?.id ?? "", skip.field, at].join("/"),
    skipped: true,
    heading: null,
    lines: [{ key: "stay", content: skipLineText(skip) }],
  }));
}
