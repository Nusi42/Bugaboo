import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { known } from "../../shared/present.ts";
import { Fact } from "./Fact.tsx";
import { SlotBlocks } from "./SlotBlocks.tsx";
import { issueSlots, type IssueBlockProps } from "./slots.ts";

export interface FieldsPanelProps {
  readonly blocks: IssueBlockProps;
  /** The panel sits under the description (a smaller screen) and is not a column of its own. */
  readonly stacked: boolean;
}

/**
 * The *Details* column (UX §3.4): what the facts bar does not hold. Reported by and the dates are
 * always read-only (nobody changes who made an issue); the own fields of WP-05 fill the slot
 * below. Empty fields the person cannot change are left out.
 */
export function FieldsPanel({ blocks, stacked }: FieldsPanelProps) {
  const { issue } = blocks;
  const [reportedBy, createdAt, updatedAt] = [
    known(issue.reported_by),
    known(issue.created_at),
    known(issue.updated_at),
  ];
  return (
    <aside
      {...partOf(part.issueDetails, { variants: [stacked && "stacked"] })}
      aria-label={t("issues.details")}
    >
      <h2 {...partOf(part.issueDetailsHeading)}>{t("issues.details")}</h2>
      {reportedBy === undefined || !blocks.form.shows("reported-by") ? null : (
        <Fact name={t("issue-field.reported-by")}>{personName(reportedBy)}</Fact>
      )}
      {createdAt === undefined ? null : (
        <Fact name={t("issues.created-on")}>{formatDateTime(createdAt)}</Fact>
      )}
      {updatedAt === undefined ? null : (
        <Fact name={t("issues.updated")}>{formatDateTime(updatedAt)}</Fact>
      )}
      <SlotBlocks slot={issueSlots.fieldsPanel} blocks={blocks} />
    </aside>
  );
}
