import { HoldWhileLoading } from "@bugaboo/ui";
import { Suspense } from "react";

import {
  issueBlocks,
  type IssueBlockEntry,
  type IssueBlockProps,
  type IssueSlot,
} from "./slots.ts";

export interface SlotBlocksProps {
  readonly slot: IssueSlot;
  readonly blocks: IssueBlockProps;
}

/**
 * One block of another package, which is loaded when it is first drawn: while it loads it holds the page with the rest
 * of its first reads, rather than taking the page away (UX §6.4).
 */
export function IssueBlockOf({
  entry,
  blocks,
}: {
  readonly entry: IssueBlockEntry;
  readonly blocks: IssueBlockProps;
}) {
  return (
    <Suspense fallback={<HoldWhileLoading />}>
      <entry.component {...blocks} />
    </Suspense>
  );
}

/** The blocks that other packages put into a slot of the issue page, in their order. */
export function SlotBlocks({ slot, blocks }: SlotBlocksProps) {
  return (
    <>
      {issueBlocks(slot).map((entry) => (
        <IssueBlockOf key={entry.id} entry={entry} blocks={blocks} />
      ))}
    </>
  );
}
