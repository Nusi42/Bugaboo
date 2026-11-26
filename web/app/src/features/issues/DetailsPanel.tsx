import { t } from "@bugaboo/messages";
import {
  DelayedSkeleton,
  NameValueList,
  partOf,
  part,
  ReadingHold,
  useHoldWhileReading,
} from "@bugaboo/ui";

import { keptScroll, scrollBox } from "../../shared/scroll-boxes.ts";
import { inLayoutOrder } from "./details-order.ts";
import { groupsOf } from "./details-groups.tsx";
import { SlotBlocks } from "./SlotBlocks.tsx";
import { issueSlots, type IssueBlockProps } from "./slots.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

export interface DetailsPanelProps {
  readonly blocks: IssueBlockProps;
  readonly changes: IssueChanges;
  /** The panel sits under the title (a smaller screen) and is not a column of its own. */
  readonly stacked: boolean;
}

/**
 * The *Details* column (UX §3.4, §4.4): the fields as name-and-value rows of one list, the own
 * fields, following, tags and labels among them (rows that other packages put below the built-in
 * ones), then what cannot be a row of that list. The rows are drawn when the layout is read, so
 * they stand once in the order of the layout and never move when it arrives (CFG-01, M3-F102).
 */
export function DetailsPanel({ blocks, changes, stacked }: DetailsPanelProps) {
  // The landmark and its heading stand at once and say they are busy; the rows, which come with reads of their own, are
  // held in it until all are read, so none pushes another (UX §6.4).
  return (
    <aside
      aria-busy={!blocks.form.ready}
      {...partOf(part.issueDetails, { variants: [stacked && "stacked"] })}
      {...keptScroll(scrollBox.issueDetails)}
      aria-label={t("issues.details")}
    >
      <div {...partOf(part.issueDetailsCard)}>
        <h2 {...partOf(part.issueDetailsHeading)}>{t("issues.details")}</h2>
        <ReadingHold>
          <DetailsRows blocks={blocks} changes={changes} />
        </ReadingHold>
      </div>
    </aside>
  );
}

/** The rows of the Details, once the layout is read, and what cannot be a row. */
function DetailsRows({ blocks, changes }: Pick<DetailsPanelProps, "blocks" | "changes">) {
  // The rows come once the layout is read: the hold waits for it.
  useHoldWhileReading(!blocks.form.ready);
  return (
    <>
      {blocks.form.ready ? (
        <NameValueList>
          {inLayoutOrder(
            blocks.form.sections.flatMap((section) => section.fields),
            groupsOf({ blocks, changes }),
          )}
        </NameValueList>
      ) : (
        <DelayedSkeleton />
      )}
      <SlotBlocks slot={issueSlots.detailSections} blocks={blocks} />
    </>
  );
}
