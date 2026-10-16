import { t } from "@bugaboo/messages";
import { NameValueList, partOf, part } from "@bugaboo/ui";

import { ChangeableRows } from "./ChangeableRows.tsx";
import { FixedRows } from "./FixedRows.tsx";
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
 * The *Details* column (UX §3.4, §4.4): the fields as name-and-value rows, then what other
 * packages put below them (the own fields of WP-05, following, labels and tags).
 */
export function DetailsPanel({ blocks, changes, stacked }: DetailsPanelProps) {
  return (
    <aside
      {...partOf(part.issueDetails, { variants: [stacked && "stacked"] })}
      aria-label={t("issues.details")}
    >
      <h2 {...partOf(part.issueDetailsHeading)}>{t("issues.details")}</h2>
      <NameValueList>
        <ChangeableRows blocks={blocks} changes={changes} />
        <FixedRows blocks={blocks} />
      </NameValueList>
      <SlotBlocks slot={issueSlots.details} blocks={blocks} />
    </aside>
  );
}
