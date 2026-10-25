import { t } from "@bugaboo/messages";
import { NameValueList, Skeleton, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { AssignedToRow, PriorityRow } from "./ChangeableRows.tsx";
import { useAfterDelay } from "../../shared/use-optimistic.ts";
import { inLayoutOrder, type FieldWord, type RowGroup } from "./details-order.ts";
import { FixedRows } from "./FixedRows.tsx";
import { SlotBlocks } from "./SlotBlocks.tsx";
import { issueBlocks, issueSlots, type IssueBlockProps } from "./slots.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

export interface DetailsPanelProps {
  readonly blocks: IssueBlockProps;
  readonly changes: IssueChanges;
  /** The panel sits under the title (a smaller screen) and is not a column of its own. */
  readonly stacked: boolean;
}

/** The keys of the built-in groups of rows: the words of their fields in the layout. */
const fields = {
  assignedTo: "assigned-to",
  priority: "priority",
  reportedBy: "reported-by",
} as const satisfies Record<string, FieldWord>;

/** The groups of rows of the Details: the built-in fields, then what other packages add. */
function groupsOf({ blocks, changes }: Pick<DetailsPanelProps, "blocks" | "changes">) {
  const own: readonly RowGroup<ReactNode>[] = [
    {
      places: [fields.assignedTo],
      node: <AssignedToRow key={fields.assignedTo} blocks={blocks} changes={changes} />,
    },
    {
      places: [fields.priority],
      node: <PriorityRow key={fields.priority} blocks={blocks} changes={changes} />,
    },
    {
      places: [fields.reportedBy],
      node: <FixedRows key={fields.reportedBy} blocks={blocks} />,
    },
  ];
  const added = issueBlocks(issueSlots.details).map((entry) => ({
    places: entry.places ?? "end",
    node: <entry.component key={entry.id} {...blocks} />,
  }));
  return [...own, ...added];
}

/**
 * The *Details* column (UX §3.4, §4.4): the fields as name-and-value rows of one list, the own
 * fields, following, tags and labels among them (rows that other packages put below the built-in
 * ones), then what cannot be a row of that list. The rows are drawn when the layout is read, so
 * they stand once in the order of the layout and never move when it arrives (CFG-01, M3-F102).
 */
export function DetailsPanel({ blocks, changes, stacked }: DetailsPanelProps) {
  const shapes = useAfterDelay(!blocks.form.ready);
  return (
    <aside
      aria-busy={!blocks.form.ready}
      {...partOf(part.issueDetails, { variants: [stacked && "stacked"] })}
      aria-label={t("issues.details")}
    >
      <h2 {...partOf(part.issueDetailsHeading)}>{t("issues.details")}</h2>
      {blocks.form.ready ? (
        <NameValueList>
          {inLayoutOrder(
            blocks.form.sections.flatMap((section) => section.fields),
            groupsOf({ blocks, changes }),
          )}
        </NameValueList>
      ) : shapes ? (
        <Skeleton />
      ) : null}
      <SlotBlocks slot={issueSlots.detailSections} blocks={blocks} />
    </aside>
  );
}
