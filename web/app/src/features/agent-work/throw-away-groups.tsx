import { t } from "@bugaboo/messages";
import { IssueId, type LongPreviewGroup } from "@bugaboo/ui";
import type { Wording } from "@bugaboo/wasm";

import { changeArrowText } from "../../shared/change-line.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { skipReasonText, type RunChange, type UndoSkip } from "../runs/index.ts";

/** What goes back, item by item: the issue is the group, each field that changes back is a line. */
export function goesBackGroups(
  wording: Wording,
  items: readonly Readonly<RunChange>[],
): readonly LongPreviewGroup[] {
  return items.map((item) => ({
    key: item.event,
    heading: <IssueId id={item.issue.readable_id} />,
    lines: item.changes.map((field) => ({
      key: field.field,
      content: changeArrowText(wording, field),
    })),
  }));
}

/** What stays of one skipped field: the issue is the group, the field and why are the line. */
export function stayGroups(skipped: readonly UndoSkip[]): readonly LongPreviewGroup[] {
  return skipped.map((skip, at) => ({
    key: ["stay", skip.issue?.id ?? "", skip.field, at].join("/"),
    skipped: true,
    heading:
      skip.issue === undefined || skip.issue === null ? null : (
        <IssueId id={skip.issue.readable_id} />
      ),
    lines: [
      {
        key: "stay",
        content: t("agent-work.throw-away-stay", {
          field: fieldNameOf({ code: skip.field, name: skip.name }),
          reason: skipReasonText(skip),
        }),
      },
    ],
  }));
}
