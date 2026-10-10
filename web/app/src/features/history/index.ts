import { addIssueBlock, issueSlots } from "../issues/index.ts";
import { HistoryBlock } from "./HistoryBlock.tsx";

export { HistoryBlock } from "./HistoryBlock.tsx";

/** Puts *History* into its slot at the end of the main column of the issue page (WP-07). */
export function registerHistoryBlock(): void {
  addIssueBlock(issueSlots.history, { id: "history", order: 0, component: HistoryBlock });
}
