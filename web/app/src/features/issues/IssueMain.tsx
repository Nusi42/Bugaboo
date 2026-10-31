import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { FormNotice } from "../layouts/index.ts";
import { DescriptionBlock } from "./DescriptionBlock.tsx";
import { historyId } from "./history-id.ts";
import { SlotBlocks } from "./SlotBlocks.tsx";
import {
  issueBlocks,
  issueSlots,
  type IssueBlockProps,
  type IssueDetail,
  type IssueSlot,
} from "./slots.ts";

export interface IssueMainProps {
  readonly detail: IssueDetail;
  readonly address: string;
  /** The blocks of other packages get these. */
  readonly blocks: IssueBlockProps;
}

/**
 * The blocks of a slot, each in a section of the main column of its own: a thin line stands
 * between two sections, and a block with nothing to show leaves an empty section that takes no
 * room (UX §4.4).
 */
function SectionBlocks({
  slot,
  blocks,
}: {
  readonly slot: IssueSlot;
  readonly blocks: IssueBlockProps;
}) {
  return issueBlocks(slot).map((entry) => (
    <div key={entry.id} {...partOf(part.issueMainSection)}>
      <entry.component {...blocks} />
    </div>
  ));
}

/** The main column: the description, then what other packages put under it (UX §4.4). */
export function IssueMain({ detail, address, blocks }: IssueMainProps) {
  return (
    <div {...partOf(part.issueMain)}>
      <FormNotice form={blocks.form} />
      {blocks.form.shows("description") ? (
        <div {...partOf(part.issueMainSection)}>
          <DescriptionBlock
            issue={detail.issue}
            address={address}
            html={detail.description_html}
            canChange={blocks.mask.changes("description")}
          />
        </div>
      ) : null}
      <SectionBlocks slot={issueSlots.ownFieldSections} blocks={blocks} />
      <SectionBlocks slot={issueSlots.subIssues} blocks={blocks} />
      <SectionBlocks slot={issueSlots.relatedIssues} blocks={blocks} />
      <SectionBlocks slot={issueSlots.files} blocks={blocks} />
      <section id={historyId} aria-label={t("history.title")} {...partOf(part.issueMainSection)}>
        <SlotBlocks slot={issueSlots.history} blocks={blocks} />
      </section>
    </div>
  );
}
