import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { FormNotice } from "../layouts/index.ts";
import { DescriptionBlock } from "./DescriptionBlock.tsx";
import { historyId } from "./history-id.ts";
import { SlotBlocks } from "./SlotBlocks.tsx";
import { issueSlots, type IssueBlockProps, type IssueDetail } from "./slots.ts";

export interface IssueMainProps {
  readonly detail: IssueDetail;
  readonly address: string;
  /** The blocks of other packages get these. */
  readonly blocks: IssueBlockProps;
}

/** The main column: the description, then what other packages put under it (UX §4.4). */
export function IssueMain({ detail, address, blocks }: IssueMainProps) {
  return (
    <div {...partOf(part.issueMain)}>
      <FormNotice form={blocks.form} />
      {blocks.form.shows("description") ? (
        <DescriptionBlock
          issue={detail.issue}
          address={address}
          html={detail.description_html}
          canChange={blocks.mask.changes("description")}
        />
      ) : null}
      <SlotBlocks slot={issueSlots.ownFieldSections} blocks={blocks} />
      <SlotBlocks slot={issueSlots.subIssues} blocks={blocks} />
      <SlotBlocks slot={issueSlots.relatedIssues} blocks={blocks} />
      <section id={historyId} aria-label={t("issues.history")}>
        <SlotBlocks slot={issueSlots.history} blocks={blocks} />
      </section>
    </div>
  );
}
