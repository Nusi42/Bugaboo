import { issueBlocks, type IssueBlockProps, type IssueSlot } from "./slots.ts";

export interface SlotBlocksProps {
  readonly slot: IssueSlot;
  readonly blocks: IssueBlockProps;
}

/** The blocks that other packages put into a slot of the issue page, in their order. */
export function SlotBlocks({ slot, blocks }: SlotBlocksProps) {
  return (
    <>
      {issueBlocks(slot).map((entry) => (
        <entry.component key={entry.id} {...blocks} />
      ))}
    </>
  );
}
